use moyu_core::base::Rect;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};

use moyu_core::core::render_command::FilterKind;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Focusable, Node, NodeBaseTrait};
use moyu_core::utils::convert::{JSValue, from_js};

#[derive(Debug, Node)]
pub struct Filter {
    pub filters: Vec<FilterKind>,

    /// Offscreen texture for rendering child nodes
    pub offscreen_view: Option<wgpu::TextureView>,

    /// Final texture after applying filters
    pub final_view: Option<wgpu::TextureView>,

    /// Intermediate texture for filter processing (ping-pong)
    pub intermediate_view: Option<wgpu::TextureView>,

    /// Rect area to capture and draw, relative to the stage
    pub rect: Option<Rect>,
    pub buffer: Option<wgpu::Buffer>,
    pub bind_group: Option<wgpu::BindGroup>,

    /// Last allocated texture size (for detecting resize)
    pub(crate) last_width: u32,
    pub(crate) last_height: u32,

    #[base]
    node_base: NodeBase,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            filters: Vec::new(),
            offscreen_view: None,
            final_view: None,
            intermediate_view: None,
            rect: None,
            buffer: None,
            bind_group: None,
            last_width: 0,
            last_height: 0,
            node_base: NodeBase::default(),
        }
    }
}

impl Filter {
    pub fn new(label: String) -> Self {
        Self {
            filters: Vec::new(),
            offscreen_view: None,
            final_view: None,
            intermediate_view: None,
            rect: None,
            buffer: None,
            bind_group: None,
            last_width: 0,
            last_height: 0,
            node_base: NodeBase::new(label),
        }
    }
}

impl Focusable for Filter {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OffscreenPassProps {
    pub filters: Option<Vec<FilterKind>>,
    // pub width: Option<u32>,
    // pub height: Option<u32>,
}

impl Node for Filter {
    #[inline]
    fn node_type(&self) -> &'static str {
        "filter"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: OffscreenPassProps = from_js(props).unwrap();

        if let Some(filters) = props.filters {
            self.filters = filters;
        }

        // if let Some(width) = props.width {
        //     self.base_mut().set_width(width);
        // }

        // if let Some(height) = props.height {
        //     self.base_mut().set_height(height);
        // }

        self.base_mut().pend_update();
    }
}
