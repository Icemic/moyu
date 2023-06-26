use std::{any::Any, fmt::Debug};

use crate::nodes::NodeBase;
#[cfg(all(not(feature = "web"), feature = "js_runtime"))]
use crate::utils::convert::JSValue;

pub trait Node: NodeBaseTrait + Send + Sync + Debug {
    fn node_type(&self) -> &'static str;

    #[cfg(all(not(feature = "web"), feature = "js_runtime"))]
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
