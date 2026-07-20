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

        self.base_mut().set_layout_size(width, height);
    }
}
