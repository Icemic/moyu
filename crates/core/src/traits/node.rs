use std::sync::Arc;
use std::{any::Any, fmt::Debug};

use anyhow::Result;
use moyu_pal::sync::RwLock;

use crate::nodes::NodeBase;
use crate::utils::convert::JSValue;

use super::{Command, EditableTarget, Focusable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowKind {
    Rendering,
}

pub trait Node: NodeBaseTrait + Debug + Send + Sync {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized;

    fn into_node_lock(self) -> crate::core::NodeLock
    where
        Self: Sized + 'static,
    {
        Arc::new(RwLock::new(Box::new(self) as Box<dyn Node>))
    }

    /// node type identifier
    fn node_type(&self) -> &'static str;

    /// identifier for the renderer to be used to render this node, \
    /// defaults to the node type.
    fn renderer_type(&self) -> &'static str {
        self.node_type()
    }

    /// method called when the properties of the node need to be updated
    fn update_properties(&mut self, _props: &mut JSValue) {
        // defaults to do nothing
    }

    /// Hook that runs before `NodeBase::update`.
    ///
    /// Wrapper-style nodes can use this to sync parent-driven state, such as
    /// inherited size, before transform and anchor calculations run.
    fn pre_update(&mut self, _parent: &NodeBase) {}

    /// Resolve this node's final layout size after its children have been measured.
    fn measure(&mut self) {
        let (width, height) = self.base().intrinsic_size();
        self.base_mut().set_layout_size(width, height);
    }

    /// Arrange direct children after this node's layout size has been resolved.
    fn arrange(&mut self) {
        for child in self.base().children() {
            child.write().base_mut().clear_layout_position();
        }
    }

    /// Whether this node contributes its layout rectangle to an auto-sized parent.
    fn participates_in_parent_measure(&self) -> bool {
        true
    }

    /// Whether this node shadows part of its subtree lifecycle for the given kind.
    ///
    /// Shadowing means the parent node is intentionally taking over that aspect
    /// of the subtree, so core traversal should stop recursing into the
    /// children for that specific kind. Wrapper nodes such as transition slots
    /// use this to temporarily replace normal child rendering with their own
    /// retained output.
    fn shadowed(&self, _kind: ShadowKind) -> bool {
        false
    }

    /// Whether this node and its subtree are ready for retained rendering.
    fn ready(&self) -> bool {
        self.children_ready()
    }

    /// Whether all children are ready for retained rendering.
    /// It will recursively check the whole subtree, so sometimes this may be a bit costly, use with caution.
    fn children_ready(&self) -> bool {
        self.base().children().iter().all(|child| {
            let child = child.read();
            child.ready()
        })
    }

    /// return Some(self) manually if you've implemented Focusable for the node
    fn as_focusable(&self) -> Option<&dyn Focusable> {
        None
    }

    /// return Some(self) manually if you've implemented EditableTarget for the node
    fn as_editable_target(&self) -> Option<&dyn EditableTarget> {
        None
    }

    /// return Some(self) manually if you've implemented EditableTarget for the node
    fn as_editable_target_mut(&mut self) -> Option<&mut dyn EditableTarget> {
        None
    }

    /// return Some(self) manually if you've implemented Command for the node
    fn as_command(&mut self) -> Option<&mut dyn Command> {
        None
    }
}

pub trait NodeBaseTrait: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn base(&self) -> &NodeBase;
    fn base_mut(&mut self) -> &mut NodeBase;
}

impl PartialEq for dyn Node {
    fn eq(&self, other: &Self) -> bool {
        self.base().id() == other.base().id()
    }
}
