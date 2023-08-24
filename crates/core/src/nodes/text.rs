use hai_macros::Node;
use huozi::layout::{LayoutStyle, TextStyle};
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextProps<'a> {
    pub text: Option<&'a str>,
    pub layout_style: Option<LayoutStyle>,
    pub text_style: Option<TextStyle>,
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

        if let Some(layout_style) = props.layout_style {
            self.layout_style = layout_style;
        }

        if let Some(text_style) = props.text_style {
            self.text_style = text_style;
        }

        // force update vertices
        self.base_mut().pend_update();
    }
}
