use hai_macros::node;
use log::warn;
use std::any::Any;
use std::sync::{Arc, Mutex};
use winit::dpi::LogicalSize;

use crate::traits::{Node, NODE_ID, NodeType};
use crate::types::{Point, PointF, Transform};

#[node]
#[derive(Debug, Default)]
pub struct Container {}

impl Container {
    pub fn new(label: String, anchor: PointF, transform: Transform) -> Self {
        let id = unsafe {
            NODE_ID += 1;
            NODE_ID
        };
        Self {
            label,
            id,
            anchor,
            translate: Point::default(),
            transform,
            transform_to_global: Transform::default(),
            children: vec![],
        }
    }
}

impl NodeType for Container {
    fn node_type(&self) -> &'static str {
        "node"
    }
}
