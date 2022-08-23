use std::{
    any::Any,
    sync::{Arc, Mutex},
};
use winit::dpi::LogicalSize;

use crate::types::{Point, PointF, Transform};

pub static mut NODE_ID: u32 = 0;

pub trait Node: NodeType {
    fn id(&self) -> &u32;
    fn label(&self) -> &String;

    fn anchor(&self) -> &PointF;
    fn translate(&self) -> &Point;
    fn transform(&self) -> &Transform;
    fn transform_to_global(&self) -> &Transform;
    fn children(&self) -> &Vec<Arc<Mutex<dyn Node + Send>>>;

    // fn anchor_mut(&mut self) -> &mut PointF;
    // fn translate_mut(&mut self) -> &mut Point;
    // fn transform_mut(&mut self) -> &mut Transform;
    // fn transform_to_global_mut(&mut self) -> &mut Transform;
    // fn children_mut(&mut self) -> &mut Vec<Arc<Mutex<dyn Node + Send>>>;

    fn node_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn get_child(&self, index: usize) -> Option<Arc<Mutex<dyn Node + Send>>>;

    fn add_child(&mut self, child: Arc<Mutex<dyn Node + Send>>);

    fn insert_child(&mut self, index: usize, child: Arc<Mutex<dyn Node + Send>>);

    fn insert_child_before(
        &mut self,
        before_child: Arc<Mutex<dyn Node + Send>>,
        child: Arc<Mutex<dyn Node + Send>>,
    );

    fn remove_child(
        &mut self,
        child: Arc<Mutex<dyn Node + Send>>,
    ) -> Option<Arc<Mutex<dyn Node + Send>>>;

    fn remove_child_at(&mut self, index: usize) -> Option<Arc<Mutex<dyn Node + Send>>>;

    fn move_to(&mut self, x: i32, y: i32);

    fn calculate_transform(
        &mut self,
        parent_transform: &Transform,
        logical_size: LogicalSize<f64>,
        scale_factor: f64,
    );
}

use core::fmt::Debug;
impl Debug for dyn Node + Send {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Series{{{}}}", 111)
    }
}

impl PartialEq for dyn Node + Send {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

pub trait NodeType {
    fn node_type(&self) -> &'static str;
}
