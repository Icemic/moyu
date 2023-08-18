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
            vertex_buffer: None,
            index_buffer: None,
            node_base: NodeBase::new(label),
            num_indices: 0,
        }
    }
}

impl Focusable for Text {
    fn contains(&self, x: f64, y: f64, _: &RendererUpdatePayload) -> bool {
        let translate = self.base().translate();

        let width = self.layout_style.box_width;
        let height = self.layout_style.box_height;

        if x > translate.x
            && x < width as f64 + translate.x
            && y > translate.y
            && y < height as f64 + translate.y
        {
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
