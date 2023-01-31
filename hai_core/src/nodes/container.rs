use hai_macros::node;
use log::warn;
use std::any::Any;
use std::sync::{Arc, Mutex};
use winit::dpi::LogicalSize;

use crate::traits::{Node, NodeType, NODE_ID};
use crate::types::{Point, Transform};

#[node]
#[derive(Debug, Default)]
pub struct Container {}

impl Container {
    pub fn new(label: String) -> Self {
        let id = unsafe {
            NODE_ID += 1;
            NODE_ID
        };
        Self {
            label,
            id,
            anchor: Point::default(),
            pivot: Point::default(),
            translate: Point::default(),
            scale: Point::one(),
            rotation: 0.,
            skew: Point::default(),

            _update_id: 0,
            _current_update_id: 1,

            transform: Transform::default(),
            global_transform: Transform::default(),
            children: vec![],
        }
    }
}

impl NodeType for Container {
    fn node_type(&self) -> &'static str {
        "node"
    }
}
