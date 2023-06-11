use std::{any::Any, fmt::Debug};

use crate::nodes::NodeBase;
use crate::utils::convert::JSValue;

pub trait Node: NodeBaseTrait + Send + Sync + Debug {
    fn node_type(&self) -> &'static str;

    fn update_properties(&mut self, _props: &mut JSValue) {
        // defaults to do nothing
    }
}

pub trait NodeBaseTrait {
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
