use std::borrow::Cow;

use anyhow::Result;
use csscolorparser::Color;
use huozi::glyph_vertices::GlyphVertices;
use huozi::layout::{LayoutDirection, LayoutStyle, SegmentGlyphSpan};
use huozi::parser::{Segment, SegmentId, ShadowStyle, StrokeStyle, TextStyle};
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use wgpu::Buffer;

use moyu_core::apply_patch;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Command, NodeEventSource};
use moyu_core::traits::{Focusable, Node, NodeBaseTrait};
use moyu_core::utils::convert::{JSValue, from_js, to_js};
use moyu_core::utils::patch::Patch;

use crate::events::TextEvent;

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
    /// glyph vertices after layout
    pub glyph_vertices: Vec<GlyphVertices>,
    /// glyph ranges of segments after layout
    pub glyph_ranges: Vec<SegmentGlyphSpan>,
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

impl Focusable for Text {}

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
    /// the writing direction of the text in the box,
    /// only `Horizontal` (right-to-left) or `Vertical` (top-to-bottom) is valid.
    #[ts(type = "'horizontal' | 'vertical'", optional)]
    pub direction: Patch<LayoutDirection>,
    /// the width of box.
    pub box_width: Patch<f64>,
    /// the height of box.
    pub box_height: Patch<f64>,
    /// the size of the glyph grid which each character be fit to, usually equals to `font_size`.
    pub glyph_grid_size: Patch<f64>,

    /* text styles */
    pub font_size: Patch<f64>,
    #[ts(type = "string", optional)]
    pub fill_color: Patch<Color>,
    pub line_height: Patch<f64>,
    pub indent: Patch<f64>,

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

        match props.print_mode {
            Patch::Set(print_mode) => self.print_mode = print_mode,
            Patch::Reset => self.print_mode = TextPrintMode::default(),
            Patch::Missing => {}
        }

        apply_patch!(props.print_speed => self.print_speed, 2.0);
        apply_patch!(props.parse_markup => self.parse_markup, true);
        apply_patch!(props.direction => self.layout_style.direction, LayoutDirection::default());
        apply_patch!(props.box_width => self.layout_style.box_width, LayoutStyle::default().box_width);
        apply_patch!(props.box_height => self.layout_style.box_height, LayoutStyle::default().box_height);
        apply_patch!(props.glyph_grid_size => self.layout_style.glyph_grid_size, LayoutStyle::default().glyph_grid_size);
        apply_patch!(props.font_size => self.text_style.font_size, TextStyle::default().font_size);
        apply_patch!(props.fill_color => self.text_style.fill_color, TextStyle::default().fill_color);
        apply_patch!(props.line_height => self.text_style.line_height, TextStyle::default().line_height);
        apply_patch!(props.indent => self.text_style.indent, TextStyle::default().indent);

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
