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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ShaderSlotSpace {
    #[default]
    Normal,
    Shader,
}

#[derive(Debug, Node)]
pub struct ShaderSlot {
    pub(crate) channel: u32,
    pub(crate) empty: bool,
    pub(crate) is_static: bool,
    /// Coordinate space for the shader, either `normal` (default) or `shader`. \
    /// `normal` means the shader will be rendered in the same coordinate space as the children,
    /// while `shader` means the shader will be rendered in a separate coordinate space where (0, 0)
    /// is the top-left corner of the shader slot and (width, height) is the bottom-right corner.
    pub(crate) space: ShaderSlotSpace,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) render_target: Option<wgpu::TextureView>,
    pub(crate) render_resolve_target: Option<wgpu::TextureView>,
    pub(crate) render_rect: Rect,
    pub(crate) render_content_origin: (f32, f32),
    pub(crate) render_children: bool,
    pub(crate) content_layout_size: (f32, f32),

    #[base]
    node_base: NodeBase,
}

impl Default for ShaderSlot {
    fn default() -> Self {
        Self {
            channel: 0,
            empty: false,
            is_static: false,
            space: ShaderSlotSpace::Normal,
            width: 0,
            height: 0,
            render_target: None,
            render_resolve_target: None,
            render_rect: Rect::default(),
            render_content_origin: (0.0, 0.0),
            render_children: true,
            content_layout_size: (0.0, 0.0),
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
    pub space: Patch<ShaderSlotSpace>,
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
            let child_pivot = child_base.pivot();
            width = width.max(child_base.translate().x - child_pivot.x * child_width + child_width);
            height =
                height.max(child_base.translate().y - child_pivot.y * child_height + child_height);
        }

        self.content_layout_size = (width, height);
        self.base_mut().set_layout_size(0.0, 0.0);
    }

    fn participates_in_parent_measure(&self) -> bool {
        false
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: ShaderSlotProps = from_js(props).unwrap();
        apply_patch!(props.channel => self.channel, 0);
        apply_patch!(props.empty => self.empty, false);
        apply_patch!(props.is_static => self.is_static, false);
        apply_patch!(props.space => self.space, ShaderSlotSpace::Normal);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_slot_keeps_zero_layout_size_without_children() {
        let mut slot = ShaderSlot::default();
        slot.base_mut().set_layout_size(320.0, 180.0);

        slot.measure();

        assert_eq!(slot.content_layout_size, (0.0, 0.0));
        assert_eq!(slot.base().layout_size(), (0.0, 0.0));
        assert!(!slot.participates_in_parent_measure());
    }
}
