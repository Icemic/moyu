use std::{any::Any, fmt::Debug};

use crate::nodes::NodeBase;
use crate::utils::convert::JSValue;

use super::Command;
use super::Focusable;

pub trait Node: NodeBaseTrait + Debug + Send + Sync {
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

    /// return Some(self) manually if you've implemented Focusable for the node
    fn as_focusable(&self) -> Option<&dyn Focusable> {
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
