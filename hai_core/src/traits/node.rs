use std::{
    any::Any,
    fmt::Debug,
    sync::{Arc, Mutex},
};
use winit::dpi::LogicalSize;

use crate::types::{Point, Transform};

use super::Renderable;

pub static mut NODE_ID: u32 = 0;

pub trait Node: NodeType + Send + Debug {
    fn id(&self) -> &u32;
    fn label(&self) -> &String;

    fn anchor(&self) -> &Point;
    fn pivot(&self) -> &Point;
    fn translate(&self) -> &Point;
    fn scale(&self) -> &Point;
    fn rotation(&self) -> &f64;
    fn skew(&self) -> &Point;

    fn set_anchor(&mut self, x: f64, y: f64);
    fn set_pivot(&mut self, x: f64, y: f64);
    fn set_translate(&mut self, x: f64, y: f64);
    fn set_scale(&mut self, x: f64, y: f64);
    fn set_rotation(&mut self, radian: f64);
    fn set_skew(&mut self, x: f64, y: f64);

    fn transform(&self) -> &Transform;
    fn global_transform(&self) -> &Transform;
    fn children(&self) -> &Vec<Arc<Mutex<dyn Node>>>;

    fn node_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn try_as_renderable(&self) -> Option<&dyn Renderable> {
        None
    }
    fn try_as_renderable_mut(&mut self) -> Option<&mut dyn Renderable> {
        None
    }

    fn get_child(&self, index: usize) -> Option<Arc<Mutex<dyn Node>>>;

    fn add_child(&mut self, child: Arc<Mutex<dyn Node>>);

    fn insert_child(&mut self, index: usize, child: Arc<Mutex<dyn Node>>);

    fn insert_child_before(
        &mut self,
        before_child: Arc<Mutex<dyn Node>>,
        child: Arc<Mutex<dyn Node>>,
    );

    fn remove_child(&mut self, child: Arc<Mutex<dyn Node>>) -> Option<Arc<Mutex<dyn Node>>>;

    fn remove_child_at(&mut self, index: usize) -> Option<Arc<Mutex<dyn Node>>>;

    fn move_to(&mut self, x: f64, y: f64);

    fn update_transform(
        &mut self,
        parent_transform: &Transform,
        logical_size: LogicalSize<f64>,
        scale_factor: f64,
        force: bool,
    );
}

impl PartialEq for dyn Node {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

pub trait NodeType {
    fn node_type(&self) -> &'static str;
}
