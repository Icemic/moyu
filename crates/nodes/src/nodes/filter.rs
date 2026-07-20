use anyhow::Result;
use moyu_core::base::Rect;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};

use moyu_core::apply_patch;
use moyu_core::core::render_command::FilterKind;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Focusable, Node, NodeBaseTrait};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_core::utils::patch::Patch;
use ts_rs::TS;

#[derive(Debug, Node)]
pub struct Filter {
    pub filters: Vec<FilterKind>,

    /// Offscreen texture for rendering child nodes
    pub offscreen_view: Option<wgpu::TextureView>,
    pub msaa_view: Option<wgpu::TextureView>,

    /// Final texture after applying filters
    pub final_view: Option<wgpu::TextureView>,

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
            msaa_view: None,
            final_view: None,
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
            msaa_view: None,
            final_view: None,
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

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct FilterProps {
    pub filters: Patch<Vec<FilterKind>>,
}

impl Node for Filter {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        let label = label.unwrap_or_default();
        Ok(Box::new(Self::new(label)))
    }

    #[inline]
    fn node_type(&self) -> &'static str {
        "filter"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: FilterProps = from_js(props).unwrap();
        apply_patch!(props.filters => self.filters, Vec::new());
        self.base_mut().pend_update();
    }

    fn measure(&mut self) {
        let mut width = 0.0_f32;
        let mut height = 0.0_f32;

        for child in self.base().children() {
            let child = child.read();
            if !child.participates_in_parent_measure() {
                continue;
            }

            let child_base = child.base();
            let (child_width, child_height) = child_base.layout_size();
            width = width.max(child_base.translate().x + child_width);
            height = height.max(child_base.translate().y + child_height);
        }

        self.base_mut().set_layout_size(width, height);
    }

    fn as_focusable(&self) -> Option<&dyn Focusable> {
        Some(self)
    }
}
