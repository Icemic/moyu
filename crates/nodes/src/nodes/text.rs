use std::borrow::Cow;

use anyhow::Result;
use csscolorparser::Color;
use huozi::glyph_vertices::UnitVertices;
use huozi::layout::{
    FirstLineIndent, Interaction, LayoutStyle, MeasureAdaptiveFirstLineIndent, ParagraphAlignment,
    RubyLineHeightMode, SegmentGlyphSpan, TabSize, TextWrapMode, WhiteSpaceMode, WritingMode,
};
use huozi::parser::{
    FontSynthesis, InlineAttachment, Segment, SegmentId, ShadowStyle, StrokeStyle, TextStyle,
};
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use wgpu::Buffer;

use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Command, NodeEventSource};
use moyu_core::traits::{Focusable, Node, NodeBaseTrait, PointerEventKind};
use moyu_core::utils::convert::{JSValue, from_js, to_js};
use moyu_core::utils::patch::Patch;
use moyu_core::{apply_patch, apply_patch_optional};
use moyu_pal::sync::Mutex;

use crate::events::{TextEvent, TextInteractionEvent};

#[derive(Debug, Default, Node)]
pub struct Text {
    /// the current text content
    pub text: String,
    /// the layout style of text, see [`LayoutStyle`] for more details.
    pub layout_style: LayoutStyle,
    /// the text style of text, see [`TextStyle`] for more details.
    pub text_style: TextStyle,
    /// the print mode of text, default is [`TextPrintMode::Instant`].
    pub print_mode: TextPrintMode,
    /// the speed of text printing,
    /// in characters per second if `print_mode` is [`TextPrintMode::Typewriter`],
    /// or lines per second if `print_mode` is [`TextPrintMode::Printer`],
    /// and it will be ignored if `print_mode` is [`TextPrintMode::Instant`].
    pub print_speed: f64,
    pub parse_markup: bool,

    pub segments: Vec<Segment<'static>>,
    /// drawable elements after layout, in glyph display order
    pub glyph_vertices: Vec<UnitVertices>,
    /// glyph ranges of segments after layout
    pub glyph_ranges: Vec<SegmentGlyphSpan>,
    /// interaction regions after layout
    pub interactions: Vec<Interaction>,
    /// interaction currently under the pointer
    hovered_interaction_id: Mutex<Option<String>>,
    /// acutal width after layout
    pub total_width: u32,
    /// acutal height after layout
    pub total_height: u32,

    pub vertex_buffer: Option<Buffer>,
    pub index_buffer: Option<Buffer>,
    pub num_indices: u32,

    /// Start time of text printing, used for typewriter or printer mode.\
    /// It will be set to `None` after printing finished.
    pub(crate) print_start_time: Option<f64>,
    /// Store the position of current printing span. It is the index of `segments`.
    pub(crate) current_range_index: usize,
    /// Store the position of next character to be printed. And also used for cursor position.
    pub(crate) cursor_position: Option<(f32, f32)>,

    #[base]
    node_base: NodeBase,
}

/// Text print mode
#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TextPrintMode {
    /// print all text at once
    #[default]
    Instant,
    /// print text character by character, like a typewriter
    Typewriter,
    /// print text line by line, like a printer
    Printer,
}

impl Text {
    pub fn new(label: String, text: &str) -> Self {
        Text {
            text: text.to_owned(),
            layout_style: LayoutStyle::default(),
            text_style: TextStyle::default(),
            print_mode: TextPrintMode::default(),
            print_speed: 2.0,
            parse_markup: true,
            total_width: 0,
            total_height: 0,
            segments: vec![],
            glyph_vertices: vec![],
            glyph_ranges: vec![],
            interactions: vec![],
            hovered_interaction_id: Mutex::new(None),
            vertex_buffer: None,
            index_buffer: None,
            num_indices: 0,
            print_start_time: None,
            current_range_index: 0,
            cursor_position: None,
            node_base: NodeBase::new(label),
        }
    }

    fn set_text(&mut self, text: String) {
        if !self.text.is_empty() && text.starts_with(&self.text) {
            let (_, appended) = text.split_at(self.text.len());
            self.segments.push(Segment {
                id: Some(SegmentId::Lite(self.segments.len() as u32)),
                content: Cow::Owned(appended.to_owned()),
            });
        } else {
            self.current_range_index = 0;
            self.segments = vec![Segment {
                id: Some(SegmentId::Lite(0)),
                content: Cow::Owned(text.clone()),
            }];
        }

        self.text = text;
        self.cursor_position = None;
    }
}

impl Focusable for Text {
    fn pointer_event(&self, x: f32, y: f32, kind: PointerEventKind) {
        let interaction_id = self.interactions.iter().find_map(|interaction| {
            interaction.areas.iter().any(|area| {
                let rect = &area.rect;
                x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
            })
            .then(|| interaction.id.clone())
        });

        if kind == PointerEventKind::Over {
            let mut hovered_interaction_id = self.hovered_interaction_id.lock();
            if *hovered_interaction_id == interaction_id {
                if let Some(id) = interaction_id {
                    drop(hovered_interaction_id);
                    self.send_event(TextEvent::Interaction(TextInteractionEvent { id, kind }));
                }
                return;
            }

            if let Some(id) = hovered_interaction_id.take() {
                self.send_event(TextEvent::Interaction(TextInteractionEvent {
                    id,
                    kind: PointerEventKind::Leave,
                }));
            }
            if let Some(id) = interaction_id {
                *hovered_interaction_id = Some(id.clone());
                self.send_event(TextEvent::Interaction(TextInteractionEvent {
                    id: id.clone(),
                    kind: PointerEventKind::Enter,
                }));
                self.send_event(TextEvent::Interaction(TextInteractionEvent { id, kind }));
            }
            return;
        }

        if kind == PointerEventKind::Leave {
            if let Some(id) = self.hovered_interaction_id.lock().take() {
                self.send_event(TextEvent::Interaction(TextInteractionEvent { id, kind }));
            }
            return;
        }

        if kind == PointerEventKind::Enter {
            let mut hovered_interaction_id = self.hovered_interaction_id.lock();
            if *hovered_interaction_id == interaction_id {
                return;
            }
            if let Some(id) = hovered_interaction_id.take() {
                self.send_event(TextEvent::Interaction(TextInteractionEvent {
                    id,
                    kind: PointerEventKind::Leave,
                }));
            }
            if let Some(id) = interaction_id {
                *hovered_interaction_id = Some(id.clone());
                self.send_event(TextEvent::Interaction(TextInteractionEvent { id, kind }));
            }
            return;
        }

        if let Some(id) = interaction_id {
            self.send_event(TextEvent::Interaction(TextInteractionEvent {
                id,
                kind,
            }));
        }
    }
}

/// Placement of the width left over by line-length quantization.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum LineLengthGridBodyAlignment {
    /// Follow the last-line alignment.
    #[default]
    Default,
    Start,
    Center,
    End,
}

impl LineLengthGridBodyAlignment {
    fn to_alignment(self) -> Option<ParagraphAlignment> {
        match self {
            Self::Default => None,
            Self::Start => Some(ParagraphAlignment::Start),
            Self::Center => Some(ParagraphAlignment::Center),
            Self::End => Some(ParagraphAlignment::End),
        }
    }
}

/**
 * To be compatible with react-spring inside JS runtime, we have to flatten the struct.
 * FIXME: But `#[serde(flatten)]` works not quite well when there are more than one `#[serde(flatten)]` in a struct.
 * So we do it manually.
 */
#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct TextProps {
    pub text: Patch<String>,
    pub print_mode: Patch<TextPrintMode>,
    pub print_speed: Patch<f64>,
    pub parse_markup: Patch<bool>,

    /* layout styles */
    /// the width of box.
    #[ts(type = "number", optional)]
    pub box_width: Patch<f64>,
    /// the height of box.
    #[ts(type = "number", optional)]
    pub box_height: Patch<f64>,

    /* paragraph styles */
    /// alignment of the last line; only effective with a finite `boxWidth`.
    #[ts(type = "'start' | 'center' | 'end'", optional)]
    pub align: Patch<ParagraphAlignment>,
    /// indent applied to the start edge of every line, measured in character widths.
    pub block_indent: Patch<f64>,
    /// first-line indent: a character count, or `auto` to follow the policy below.
    #[ts(type = "number | 'auto'", optional)]
    pub indent: Patch<FirstLineIndent>,
    /// adaptive first-line indent policy; only used when `indent` is `auto`.
    #[ts(
        type = "{ shortBelowEm: number, shortEm: number, longEm: number }",
        optional
    )]
    pub first_line_indent_policy: Patch<MeasureAdaptiveFirstLineIndent>,
    /// how source white space is processed.
    #[ts(
        type = "'collapse' | 'preserve-breaks' | 'preserve' | 'break-spaces'",
        optional
    )]
    pub white_space: Patch<WhiteSpaceMode>,
    /// whether soft wrapping is allowed.
    #[ts(type = "'wrap' | 'no-wrap'", optional)]
    pub text_wrap: Patch<TextWrapMode>,
    /// tab stop interval as a multiple of the space advance.
    pub tab_size: Patch<f64>,
    /// writing mode; currently passed through without affecting layout.
    #[ts(type = "'horizontal-tb' | 'vertical-rl'", optional)]
    pub writing_mode: Patch<WritingMode>,
    /// whether line length is quantized to whole multiples of the font size.
    pub line_length_grid: Patch<bool>,
    /// placement of the leftover width after quantization; `default` follows `align`.
    pub line_length_grid_body_alignment: Patch<LineLengthGridBodyAlignment>,
    /// how ruby expands line height when space is insufficient.
    #[ts(type = "'per-line' | 'uniform-paragraph'", optional)]
    pub ruby_line_height_mode: Patch<RubyLineHeightMode>,
    /// minimum clearance between inline objects and adjacent lines, in em.
    pub inline_object_minimum_clearance: Patch<f64>,
    /// clearance between emphasis dots and the marked glyphs, in em.
    pub emphasis_dot_gap: Patch<f64>,

    /* text styles */
    pub font_size: Patch<f64>,
    /// preferred font families, first one wins when available.
    #[ts(type = "string[]", optional)]
    pub font_families: Patch<Vec<String>>,
    /// OpenType weight axis, 400 for regular and 700 for bold.
    pub font_weight: Patch<i32>,
    /// use an italic or oblique typeface when the family offers one.
    pub italic: Patch<bool>,
    /// whether software synthesis may fake a missing weight or slant.
    #[ts(type = "'none' | 'weight' | 'style' | 'all'", optional)]
    pub font_synthesis: Patch<FontSynthesis>,
    /// explicit baseline offset in pixels; positive values move the text down.
    pub baseline_shift: Patch<f32>,
    /// how the run attaches to the preceding text boundary.
    #[ts(type = "'none' | 'previous'", optional)]
    pub inline_attachment: Patch<InlineAttachment>,
    /// BCP 47 language tag used for shaping and line breaking.
    pub locale: Patch<String>,
    #[ts(type = "string", optional)]
    pub fill_color: Patch<Color>,
    pub line_height: Patch<f64>,

    pub stroke: Patch<bool>,
    pub shadow: Patch<bool>,

    #[ts(type = "string", optional)]
    pub stroke_color: Patch<Color>,
    pub stroke_width: Patch<f32>,

    #[ts(type = "string", optional)]
    pub shadow_color: Patch<Color>,
    pub shadow_offset_x: Patch<f32>,
    pub shadow_offset_y: Patch<f32>,
    pub shadow_blur: Patch<f32>,
    pub shadow_width: Patch<f32>,
}

impl Node for Text {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        let label = label.unwrap_or_default();
        Ok(Box::new(Self::new(label, "")))
    }

    #[inline]
    fn node_type(&self) -> &'static str {
        "text"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: TextProps = match from_js(props) {
            Ok(props) => props,
            Err(e) => {
                log::error!("Error parsing props: {:?}", e);
                return;
            }
        };

        match props.text {
            Patch::Set(text) => {
                self.set_text(text);

                if self.print_start_time.is_none() {
                    // set to 0 to tell renderer start printing, its value will be updated to real time in renderer.
                    self.print_start_time = Some(0.);
                }
            }
            Patch::Reset => {
                self.text = "".to_string();
                self.segments = vec![];
                self.current_range_index = 0;
                self.print_start_time = None;
            }
            Patch::Missing => {}
        }

        apply_patch!(props.print_mode => self.print_mode, TextPrintMode::default());
        apply_patch!(props.print_speed => self.print_speed, 2.0);
        apply_patch!(props.parse_markup => self.parse_markup, true);
        apply_patch_optional!(props.box_width => self.layout_style.box_width, None);
        apply_patch_optional!(props.box_height => self.layout_style.box_height, None);

        apply_patch!(props.font_size => self.text_style.font_size, TextStyle::default().font_size);
        apply_patch!(props.font_families => self.text_style.font_families, TextStyle::default().font_families);
        apply_patch!(props.font_weight => self.text_style.font_weight, TextStyle::default().font_weight);
        apply_patch!(props.italic => self.text_style.italic, TextStyle::default().italic);
        apply_patch!(props.font_synthesis => self.text_style.font_synthesis, TextStyle::default().font_synthesis);
        apply_patch!(props.baseline_shift => self.text_style.baseline_shift, TextStyle::default().baseline_shift);
        apply_patch!(props.inline_attachment => self.text_style.inline_attachment, TextStyle::default().inline_attachment);
        apply_patch!(props.locale => self.text_style.locale, TextStyle::default().locale);
        apply_patch!(props.fill_color => self.text_style.fill_color, TextStyle::default().fill_color);
        apply_patch!(props.line_height => self.layout_style.line_height, LayoutStyle::default().line_height);
        apply_patch!(props.align => self.layout_style.align, LayoutStyle::default().align);
        apply_patch!(props.block_indent => self.layout_style.block_indent, LayoutStyle::default().block_indent);
        apply_patch!(props.indent => self.layout_style.indent, LayoutStyle::default().indent);
        apply_patch!(props.first_line_indent_policy => self.layout_style.first_line_indent_policy, LayoutStyle::default().first_line_indent_policy);
        apply_patch!(props.white_space => self.layout_style.white_space, LayoutStyle::default().white_space);
        apply_patch!(props.text_wrap => self.layout_style.text_wrap, LayoutStyle::default().text_wrap);
        apply_patch!(props.tab_size => |v| {
            self.layout_style.tab_size = TabSize::SpaceAdvanceMultiple(v);
        }, 8.0);
        apply_patch!(props.writing_mode => self.layout_style.writing_mode, LayoutStyle::default().writing_mode);
        apply_patch!(props.line_length_grid => self.layout_style.line_length_grid.enabled, LayoutStyle::default().line_length_grid.enabled);
        apply_patch!(props.line_length_grid_body_alignment => |v| {
            self.layout_style.line_length_grid.body_alignment = v.to_alignment();
        }, LineLengthGridBodyAlignment::Default);
        apply_patch!(props.ruby_line_height_mode => self.layout_style.ruby_line_height_mode, LayoutStyle::default().ruby_line_height_mode);
        apply_patch!(props.inline_object_minimum_clearance => self.layout_style.inline_object_minimum_clearance, LayoutStyle::default().inline_object_minimum_clearance);
        apply_patch!(props.emphasis_dot_gap => self.layout_style.emphasis_dot_gap, LayoutStyle::default().emphasis_dot_gap);

        // stroke and shadow style must be updated after switch turn on, otherwise it will be default value.

        match props.stroke {
            Patch::Set(stroke) => {
                if stroke {
                    self.text_style.stroke = Some(StrokeStyle::default());
                } else {
                    self.text_style.stroke = None;
                }
            }
            Patch::Reset => self.text_style.stroke = None,
            Patch::Missing => {}
        }

        match props.shadow {
            Patch::Set(shadow) => {
                if shadow {
                    self.text_style.shadow = Some(ShadowStyle::default());
                } else {
                    self.text_style.shadow = None;
                }
            }
            Patch::Reset => self.text_style.shadow = None,
            Patch::Missing => {}
        }

        if let Some(stroke) = self.text_style.stroke.as_mut() {
            apply_patch!(props.stroke_color => stroke.stroke_color, StrokeStyle::default().stroke_color);
            apply_patch!(props.stroke_width => stroke.stroke_width, StrokeStyle::default().stroke_width);
        }

        if let Some(shadow) = self.text_style.shadow.as_mut() {
            apply_patch!(props.shadow_color => shadow.shadow_color, ShadowStyle::default().shadow_color);
            apply_patch!(props.shadow_offset_x => shadow.shadow_offset_x, ShadowStyle::default().shadow_offset_x);
            apply_patch!(props.shadow_offset_y => shadow.shadow_offset_y, ShadowStyle::default().shadow_offset_y);
            apply_patch!(props.shadow_blur => shadow.shadow_blur, ShadowStyle::default().shadow_blur);
            apply_patch!(props.shadow_width => shadow.shadow_width, ShadowStyle::default().shadow_width);
        }

        // force update vertices
        self.cursor_position = None;
        self.base_mut().pend_prepare();
    }

    fn as_focusable(&self) -> Option<&dyn Focusable> {
        Some(self)
    }

    fn as_command(&mut self) -> Option<&mut dyn Command> {
        Some(self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "subCommand")]
#[derive(TS)]
#[ts(export, optional_fields)]
pub enum TextCommand {
    SetText { text: String, instant: Option<bool> },
    FinishPrinting,
    GetCursorPosition,
}

impl Command for Text {
    fn execute(&mut self, _payload: &mut JSValue) -> anyhow::Result<Option<JSValue>> {
        let payload: TextCommand = from_js(_payload)?;
        match payload {
            TextCommand::SetText { text, instant } => {
                self.set_text(text);
                // set to 0 to tell renderer start printing, its value will be updated to real time in renderer.
                self.print_start_time = Some(0.);
                if let Some(instant) = instant {
                    if instant {
                        self.print_start_time = None;
                    }
                }
                self.base_mut().pend_prepare();
            }
            TextCommand::FinishPrinting => {
                // Set to f64::MIN to make renderer feel it's finished.
                // Cannot set to None which leads to lost of essential event sending.
                self.print_start_time = Some(f64::MIN);
                self.base_mut().pend_update();
            }
            TextCommand::GetCursorPosition => {
                return Ok(Some(to_js(&self.cursor_position)?));
            }
        }

        Ok(None)
    }
}

impl NodeEventSource for Text {
    type Event = TextEvent;
}
