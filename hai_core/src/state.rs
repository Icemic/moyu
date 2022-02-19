use std::sync::{Arc, Mutex};
use winit::{event::Event, window::Window};

use crate::node::Node;

pub struct State<'a> {
    pub pending_events: Arc<Mutex<Vec<Event<'a, ()>>>>,
    pub pending_updates: Arc<Mutex<Vec<()>>>,
    pub pending_renderable:
        Arc<Mutex<Vec<(wgpu::BindGroup, wgpu::Buffer, wgpu::Buffer, u32, u32)>>>,
    pub root_node: Arc<Mutex<Node>>,
}

impl<'a> State<'a> {
    pub fn new(window: &Window) -> Self {
        // create root node
        let root_node = Node::new(
            "Root Node".to_string(),
            Default::default(),
            Default::default(),
        );

        Self {
            pending_events: Default::default(),
            pending_updates: Default::default(),
            pending_renderable: Default::default(),
            root_node: Arc::new(Mutex::new(root_node)),
        }
    }
}
