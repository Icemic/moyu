use std::{any::Any, fmt::Debug};

use crate::nodes::NodeBase;
#[cfg(all(not(feature = "web"), feature = "js_runtime"))]
use crate::utils::convert::JSValue;

#[cfg(all(not(feature = "web"), feature = "js_runtime"))]
use super::Command;
use super::Focusable;

pub trait Node: NodeBaseTrait + Send + Sync + Debug {
    fn node_type(&self) -> &'static str;

    #[cfg(all(not(feature = "web"), feature = "js_runtime"))]
    fn update_properties(&mut self, _props: &mut JSValue) {
        // defaults to do nothing
    }

    /// return Some(self) manually if you've implemented Focusable for the node
    fn as_focusable(&self) -> Option<&dyn Focusable> {
        None
    }

    /// return Some(self) manually if you've implemented Command for the node
    #[cfg(all(not(feature = "web"), feature = "js_runtime"))]
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
