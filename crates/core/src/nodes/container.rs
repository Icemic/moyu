use std::any::Any;

use crate::traits::{GetNodeBase, Node, NodeType, UpdateProps};

use super::NodeBase;

#[derive(Debug, Default)]
pub struct Container {
    node_base: NodeBase,
}

impl Container {
    pub fn new(label: String) -> Self {
        Self {
            node_base: NodeBase::new(label),
        }
    }
}

impl NodeType for Container {
    fn node_type(&self) -> &'static str {
        "node"
    }
}

impl UpdateProps for Container {}

impl GetNodeBase for Container {
    #[inline]
    fn base(&self) -> &NodeBase {
        &self.node_base
    }

    #[inline]
    fn base_mut(&mut self) -> &mut NodeBase {
        &mut self.node_base
    }
}

impl Node for Container {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
