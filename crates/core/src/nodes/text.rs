use hai_macros::Node;
use huozi::layout::{Color, LayoutDirection, LayoutStyle, ShadowStyle, StrokeStyle, TextStyle};
use serde::{Deserialize, Serialize};
use wgpu::Buffer;

use crate::traits::{Focusable, Node, NodeBaseTrait, RendererUpdatePayload};
#[cfg(all(not(feature = "web"), feature = "js_runtime"))]
use crate::utils::convert::{from_js, JSValue};

use super::NodeBase;

#[derive(Debug, Default, Node)]
pub struct Text {
    pub text: String,
    pub layout_style: LayoutStyle,
    pub text_style: TextStyle,

    /// acutal width after layout
    pub total_width: u32,
    /// acutal height after layout
    pub total_height: u32,

    pub vertex_buffer: Option<Buffer>,
    pub index_buffer: Option<Buffer>,
    pub num_indices: u32,

    #[base]
    node_base: NodeBase,
}

impl Text {
    pub fn new(label: String, text: &str) -> Self {
        Text {
            text: text.to_owned(),
            layout_style: LayoutStyle::default(),
            text_style: TextStyle::default(),
            total_width: 0,
            total_height: 0,
            vertex_buffer: None,
            index_buffer: None,
            node_base: NodeBase::new(label),
            num_indices: 0,
        }
    }
}

impl Focusable for Text {
    fn contains(&self, x: f32, y: f32, _: &RendererUpdatePayload) -> bool {
        let width = self.total_width;
        let height = self.total_height;

        let offset_x = self.base().anchor().x * width as f32;
        let offset_y = self.base().anchor().y * height as f32;

        let x = x + offset_x;
        let y = y + offset_y;

        if x > 0. && x < width as f32 && y > 0. && y < height as f32 {
            return true;
        }

        false
    }
}

/**
 * To be compatible with react-spring inside JS runtime, we have to flatten the struct.
 * FIXME: But `#[serde(flatten)]` works not quite well when there are more than one `#[serde(flatten)]` in a struct.
 * So we do it manually.
 */
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextProps<'a> {
    pub text: Option<&'a str>,

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

impl Default for TextProps<'_> {
    fn default() -> Self {
        let layout_style_default: LayoutStyle = LayoutStyle::default();
        let text_style_default: TextStyle = TextStyle::default();
        let stroke_style_default: StrokeStyle = StrokeStyle::default();
        let shadow_style_default: ShadowStyle = ShadowStyle::default();

        Self {
            text: None,
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

    #[cfg(all(not(feature = "web"), feature = "js_runtime"))]
    fn update_properties(&mut self, props: &mut JSValue) {
        let props: TextProps = from_js(props).unwrap();

        if let Some(text) = props.text {
            self.text = text.to_owned();
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
}
