use std::borrow::Cow;

use csscolorparser::Color;
use huozi::glyph_vertices::GlyphVertices;
use huozi::layout::{LayoutDirection, LayoutStyle, SegmentGlyphSpan};
use huozi::parser::{Segment, SegmentId, ShadowStyle, StrokeStyle, TextStyle};
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use wgpu::Buffer;

use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Command, NodeEventSource};
use moyu_core::traits::{Focusable, Node, NodeBaseTrait};
use moyu_core::utils::convert::{JSValue, from_js, to_js};

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
#[derive(Debug, Default, Serialize, Deserialize)]
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
}

impl Focusable for Text {}

/**
 * To be compatible with react-spring inside JS runtime, we have to flatten the struct.
 * FIXME: But `#[serde(flatten)]` works not quite well when there are more than one `#[serde(flatten)]` in a struct.
 * So we do it manually.
 */
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextProps {
    pub text: Option<String>,
    pub print_mode: Option<TextPrintMode>,
    pub print_speed: Option<f64>,

    /* layout styles */
    /// the writing direction of the text in the box,
    /// only `Horizontal` (right-to-left) or `Vertical` (top-to-bottom) is valid.
    pub direction: Option<LayoutDirection>,
    /// the width of box.
    pub box_width: Option<f64>,
    /// the height of box.
    pub box_height: Option<f64>,
    /// the size of the glyph grid which each character be fit to, usually equals to `font_size`.
    pub glyph_grid_size: Option<f64>,

    /* text styles */
    pub font_size: Option<f64>,
    pub fill_color: Option<Color>,
    pub line_height: Option<f64>,
    pub indent: Option<f64>,

    pub stroke: Option<bool>,
    pub shadow: Option<bool>,

    pub stroke_color: Option<Color>,
    pub stroke_width: Option<f32>,

    pub shadow_color: Option<Color>,
    pub shadow_offset_x: Option<f32>,
    pub shadow_offset_y: Option<f32>,
    pub shadow_blur: Option<f32>,
    pub shadow_width: Option<f32>,
}

impl Default for TextProps {
    fn default() -> Self {
        let layout_style_default: LayoutStyle = LayoutStyle::default();
        let text_style_default: TextStyle = TextStyle::default();
        let stroke_style_default: StrokeStyle = StrokeStyle::default();
        let shadow_style_default: ShadowStyle = ShadowStyle::default();

        Self {
            text: None,
            print_mode: None,
            print_speed: None,
            direction: Some(layout_style_default.direction),
            box_width: Some(layout_style_default.box_width),
            box_height: Some(layout_style_default.box_height),
            glyph_grid_size: Some(layout_style_default.glyph_grid_size),
            font_size: Some(text_style_default.font_size),
            fill_color: Some(text_style_default.fill_color),
            line_height: Some(text_style_default.line_height),
            indent: Some(text_style_default.indent),
            stroke: Some(false),
            shadow: Some(false),
            stroke_color: Some(stroke_style_default.stroke_color),
            stroke_width: Some(stroke_style_default.stroke_width),
            shadow_color: Some(shadow_style_default.shadow_color),
            shadow_offset_x: Some(shadow_style_default.shadow_offset_x),
            shadow_offset_y: Some(shadow_style_default.shadow_offset_y),
            shadow_blur: Some(shadow_style_default.shadow_blur),
            shadow_width: Some(shadow_style_default.shadow_width),
        }
    }
}

impl Node for Text {
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

        if let Some(text) = props.text {
            let prev_text = self.text.clone();

            if !prev_text.is_empty() && text.starts_with(&prev_text) {
                let (_, cur) = text.split_at(prev_text.len());
                self.segments.push(Segment {
                    id: Some(SegmentId::Lite(self.segments.len() as u32)),
                    content: Cow::Owned(cur.to_owned()),
                });
            } else {
                self.current_range_index = 0;
                self.segments = vec![Segment {
                    id: Some(SegmentId::Lite(0)),
                    content: Cow::Owned(text.clone()),
                }];
            };

            self.text = text;

            if self.print_start_time.is_none() {
                // set to 0 to tell renderer start printing, its value will be updated to real time in renderer.
                self.print_start_time = Some(0.);
            }
        }

        if let Some(print_mode) = props.print_mode {
            self.print_mode = print_mode;
        }

        if let Some(print_speed) = props.print_speed {
            self.print_speed = print_speed;
        }

        if let Some(direction) = props.direction {
            self.layout_style.direction = direction;
        }

        if let Some(box_width) = props.box_width {
            self.layout_style.box_width = box_width;
        }

        if let Some(box_height) = props.box_height {
            self.layout_style.box_height = box_height;
        }

        if let Some(glyph_grid_size) = props.glyph_grid_size {
            self.layout_style.glyph_grid_size = glyph_grid_size;
        }

        if let Some(font_size) = props.font_size {
            self.text_style.font_size = font_size;
        }

        if let Some(fill_color) = props.fill_color {
            self.text_style.fill_color = fill_color;
        }

        if let Some(line_height) = props.line_height {
            self.text_style.line_height = line_height;
        }

        if let Some(indent) = props.indent {
            self.text_style.indent = indent;
        }

        // stroke and shadow style must be updated after switch turn on, otherwise it will be default value.

        if let Some(stroke) = props.stroke {
            if stroke {
                self.text_style.stroke = Some(StrokeStyle::default());
            } else {
                self.text_style.stroke = None;
            }
        }

        if let Some(shadow) = props.shadow {
            if shadow {
                self.text_style.shadow = Some(ShadowStyle::default());
            } else {
                self.text_style.shadow = None;
            }
        }

        if let Some(stroke) = self.text_style.stroke.as_mut() {
            if let Some(stroke_color) = props.stroke_color {
                stroke.stroke_color = stroke_color;
            }

            if let Some(stroke_width) = props.stroke_width {
                stroke.stroke_width = stroke_width;
            }
        }

        if let Some(shadow) = self.text_style.shadow.as_mut() {
            if let Some(shadow_color) = props.shadow_color {
                shadow.shadow_color = shadow_color;
            }

            if let Some(shadow_offset_x) = props.shadow_offset_x {
                shadow.shadow_offset_x = shadow_offset_x;
            }

            if let Some(shadow_offset_y) = props.shadow_offset_y {
                shadow.shadow_offset_y = shadow_offset_y;
            }

            if let Some(shadow_blur) = props.shadow_blur {
                shadow.shadow_blur = shadow_blur;
            }

            if let Some(shadow_width) = props.shadow_width {
                shadow.shadow_width = shadow_width;
            }
        }

        // force update vertices
        self.base_mut().pend_update();
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
pub enum TextCommmad {
    SetText { text: String, instant: Option<bool> },
    FinishPrinting,
    GetCursorPosition,
}

impl Command for Text {
    fn execute(&mut self, _payload: &mut JSValue) -> anyhow::Result<Option<JSValue>> {
        let payload: TextCommmad = from_js(_payload)?;
        match payload {
            TextCommmad::SetText { text, instant } => {
                self.text = text;
                // set to 0 to tell renderer start printing, its value will be updated to real time in renderer.
                self.print_start_time = Some(0.);
                if let Some(instant) = instant {
                    if instant {
                        self.print_start_time = None;
                    }
                }
                self.base_mut().pend_update();
            }
            TextCommmad::FinishPrinting => {
                // Set to f64::MIN to make renderer feel it's finished.
                // Cannot set to None which leads to lost of essential event sending.
                self.print_start_time = Some(f64::MIN);
                self.base_mut().pend_update();
            }
            TextCommmad::GetCursorPosition => {
                return Ok(Some(to_js(&self.cursor_position)?));
            }
        }

        Ok(None)
    }
}

impl NodeEventSource for Text {
    type Event = TextEvent;
}
