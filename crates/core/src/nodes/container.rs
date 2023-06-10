use hai_macros::Node;

use crate::traits::{Node, NodeBaseTrait};

use super::NodeBase;

#[derive(Debug, Default, Node)]
pub struct Container {
    #[base]
    node_base: NodeBase,
}

impl Container {
    pub fn new(label: String) -> Self {
        Self {
            node_base: NodeBase::new(label),
        }
    }
}

impl Node for Container {
    #[inline]
    fn node_type(&self) -> &'static str {
        "node"
    }
}
