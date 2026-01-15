use anyhow::Result;
use moyu_macros::Node;

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
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        let label = label.unwrap_or_default();
        Ok(Box::new(Self::new(label)))
    }

    #[inline]
    fn node_type(&self) -> &'static str {
        "node"
    }
}
