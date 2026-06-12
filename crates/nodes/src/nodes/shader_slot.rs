use anyhow::Result;
use moyu_core::apply_patch;
use moyu_core::base::Rect;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Node, NodeBaseTrait, ShadowKind};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_core::utils::patch::Patch;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Node)]
pub struct ShaderSlot {
    pub(crate) channel: u32,
    pub(crate) empty: bool,
    pub(crate) is_static: bool,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) render_target: Option<wgpu::TextureView>,
    pub(crate) render_rect: Rect,
    pub(crate) render_children: bool,

    #[base]
    node_base: NodeBase,
}

impl Default for ShaderSlot {
    fn default() -> Self {
        Self {
            channel: 0,
            empty: false,
            is_static: false,
            width: 0,
            height: 0,
            render_target: None,
            render_rect: Rect::default(),
            render_children: true,
            node_base: NodeBase::default(),
        }
    }
}

impl ShaderSlot {
    pub fn new(label: String) -> Self {
        Self {
            node_base: NodeBase::new(label),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct ShaderSlotProps {
    pub channel: Patch<u32>,
    pub empty: Patch<bool>,
    #[serde(rename = "static")]
    pub is_static: Patch<bool>,
    pub width: Patch<u32>,
    pub height: Patch<u32>,
}

impl Node for ShaderSlot {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        let label = label.unwrap_or_default();
        Ok(Box::new(Self::new(label)))
    }

    fn node_type(&self) -> &'static str {
        "shader-slot"
    }

    fn pre_update(&mut self, parent: &NodeBase) {
        let width = *parent.width();
        let height = *parent.height();

        if *self.base().width() != width || *self.base().height() != height {
            self.base_mut().set_size(width, height);
        }
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: ShaderSlotProps = from_js(props).unwrap();
        apply_patch!(props.channel => self.channel, 0);
        apply_patch!(props.empty => self.empty, false);
        apply_patch!(props.is_static => self.is_static, false);
        apply_patch!(props.width => self.width, 0);
        apply_patch!(props.height => self.height, 0);
        self.base_mut().pend_update();
    }

    fn shadowed(&self, kind: ShadowKind) -> bool {
        match kind {
            ShadowKind::Rendering => !self.render_children,
        }
    }
}
