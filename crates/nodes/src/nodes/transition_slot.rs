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
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum TransitionSlotPhase {
    From,
    #[default]
    To,
}

#[derive(Debug, Node)]
pub struct TransitionSlot {
    // Semantic role of this subtree inside a transition container.
    pub(crate) phase: TransitionSlotPhase,

    // Runtime state injected by the parent container for the current frame.
    // `render_target == Some(..)` means this subtree should render into a
    // retained offscreen texture instead of the parent target.
    // `render_rect` is the logical viewport used for that pass.
    // `render_children` controls whether rendering traversal should recurse
    // into the subtree at all. When false, the container is expected to show
    // retained output for this slot instead of fresh child rendering.
    pub(crate) render_target: Option<wgpu::TextureView>,
    pub(crate) render_rect: Rect,
    pub(crate) render_children: bool,

    #[base]
    node_base: NodeBase,
}

impl Default for TransitionSlot {
    fn default() -> Self {
        Self {
            phase: TransitionSlotPhase::To,
            render_target: None,
            render_rect: Rect::default(),
            render_children: true,
            node_base: NodeBase::default(),
        }
    }
}

impl TransitionSlot {
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
pub struct TransitionSlotProps {
    pub phase: Patch<TransitionSlotPhase>,
}

impl Node for TransitionSlot {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        let label = label.unwrap_or_default();
        Ok(Box::new(Self::new(label)))
    }

    fn node_type(&self) -> &'static str {
        "transition_slot"
    }

    fn pre_update(&mut self, parent: &NodeBase) {
        let width = *parent.width();
        let height = *parent.height();

        if *self.base().width() != width || *self.base().height() != height {
            self.base_mut().set_size(width, height);
        }
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: TransitionSlotProps = from_js(props).unwrap();
        apply_patch!(props.phase => self.phase, TransitionSlotPhase::To);
        self.base_mut().pend_update();
    }

    fn shadowed(&self, kind: ShadowKind) -> bool {
        match kind {
            // The slot stays in the node tree, but rendering traversal can be
            // suspended while the container shows retained output instead. This
            // is what lets snapshot/live control rendering without unmounting
            // the subtree.
            ShadowKind::Rendering => !self.render_children,
        }
    }
}
