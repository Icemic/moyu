use std::{cell::RefCell, rc::Rc};
use wgpu::Label;
use winit::dpi::LogicalSize;

use crate::{
    sprite::Sprite,
    types::{Point, PointF, Transform},
};

#[derive(Debug)]
pub enum NodeLike<'a> {
    Node(Node<'a>),
    Sprite(Sprite<'a>),
}

static mut NODE_ID: u32 = 0;

#[derive(Debug)]
pub struct Node<'a> {
    /// Debug label
    pub label: Label<'a>,
    /// id
    pub id: u32,
    /// anchor point
    pub anchor: PointF,
    /// translate relative to parent
    pub translate: Point,
    /// transform matrix relative to parent
    pub transform: Transform,
    /// transform matrix relative to global
    pub transform_to_global: Transform,
    /// children
    pub children: Vec<Rc<RefCell<NodeLike<'a>>>>,
}

impl<'a> Node<'a> {
    pub fn new(label: Label<'a>, anchor: PointF, transform: Transform) -> Self {
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

    #[allow(dead_code)]
    pub fn get_child(&self, index: usize) -> Option<Rc<RefCell<NodeLike<'a>>>> {
        if let Some(child) = self.children.get(index) {
            return Some(Rc::clone(child));
        }
        None
    }

    pub fn add_child(&mut self, child: NodeLike<'a>) {
        self.children.push(Rc::new(RefCell::new(child)));
    }

    #[allow(dead_code)]
    pub fn insert_child(&mut self, index: usize, child: NodeLike<'a>) {
        self.children.insert(index, Rc::new(RefCell::new(child)));
    }

    #[allow(dead_code)]
    pub fn remove_child(
        &mut self,
        child: Rc<RefCell<NodeLike<'a>>>,
    ) -> Option<Rc<RefCell<NodeLike<'a>>>> {
        if let Some(index) = self
            .children
            .iter()
            .position(|item| item.as_ptr() == child.as_ptr())
        {
            return Some(self.children.remove(index));
        }
        None
    }

    #[allow(dead_code)]
    pub fn remove_child_at(&mut self, index: usize) -> Option<Rc<RefCell<NodeLike<'a>>>> {
        if index < self.children.len() {
            return Some(self.children.remove(index));
        }
        None
    }

    pub fn move_to(&mut self, x: i32, y: i32) {
        self.translate.x = x;
        self.translate.y = y;
    }

    pub fn calculate_transform(
        &mut self,
        parent_transform: &Transform,
        logical_size: LogicalSize<f64>,
        scale_factor: f64,
    ) {
        let x = self.translate.x;
        let y = self.translate.y;

        // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed
        let tx = (x as f64 * scale_factor) / (logical_size.width * scale_factor) * 2.;
        let ty = (y as f64 * scale_factor) / (logical_size.height * scale_factor) * 2.;

        self.transform.tx = tx;
        self.transform.ty = ty;

        // TODO: rotate, scale and skew

        // refresh global transform matrix
        let mut transform_to_global = parent_transform.clone();
        transform_to_global.multiply(self.transform);
        self.transform_to_global = transform_to_global;
    }
}
