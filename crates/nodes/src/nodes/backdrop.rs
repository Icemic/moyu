use moyu_core::base::Rect;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};

use moyu_core::core::render_command::FilterKind;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Focusable, Node, NodeBaseTrait};
use moyu_core::utils::convert::{JSValue, from_js};

#[derive(Debug, Node)]
pub struct Backdrop {
    pub filters: Vec<FilterKind>,

    /// Source texture for capturing backdrop (before filter)
    pub source_view: Option<wgpu::TextureView>,

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

impl Default for Backdrop {
    fn default() -> Self {
        Self {
            filters: Vec::new(),
            source_view: None,
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

impl Backdrop {
    pub fn new(label: String) -> Self {
        Self {
            filters: vec![],
            source_view: None,
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

impl Focusable for Backdrop {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackdropProps {
    pub filters: Option<Vec<FilterKind>>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl Node for Backdrop {
    #[inline]
    fn node_type(&self) -> &'static str {
        "backdrop"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: BackdropProps = from_js(props).unwrap();

        if let Some(filters) = props.filters {
            self.filters = filters;
        }

        if let Some(width) = props.width {
            self.base_mut().set_width(width);
        }

        if let Some(height) = props.height {
            self.base_mut().set_height(height);
        }

        self.base_mut().pend_update();
    }
}
