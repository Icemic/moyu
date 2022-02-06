use wgpu::Label;

use crate::{
    sprite::Sprite,
    types::{PointF, Transform},
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
    /// transform matrix
    pub transform: Transform,
    /// children
    pub children: Vec<NodeLike<'a>>,
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
            transform,
            children: vec![],
        }
    }

    pub fn add_child(&mut self, child: NodeLike<'a>) {
        self.children.push(child);
    }

    pub fn insert_child(&mut self, index: usize, child: NodeLike<'a>) {
        self.children.insert(index, child);
    }

    pub fn remove_child(&mut self, child: NodeLike<'a>) -> Option<NodeLike<'a>> {
        if let Some(index) = match child {
            NodeLike::Node(node) => self.children.iter().position(|item| {
                if let NodeLike::Node(n) = item {
                    return n.id == node.id;
                }
                false
            }),
            NodeLike::Sprite(node) => self.children.iter().position(|item| {
                if let NodeLike::Sprite(n) = item {
                    return n.id == node.id;
                }
                false
            }),
        } {
            return Some(self.remove_child_at(index));
        }

        None
    }

    pub fn remove_child_at(&mut self, index: usize) -> NodeLike<'a> {
        self.children.remove(index)
    }

    pub fn move_to(&mut self, tx: f64, ty: f64) {
        self.transform.tx = (tx * 1.) / (1280. * 1.5) * 2.;
        self.transform.ty = (ty * 1.) / (720. * 1.5) * 2.;
    }
}
