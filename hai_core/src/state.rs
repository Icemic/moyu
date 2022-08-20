use log::debug;
use std::sync::{Arc, Mutex};
use wgpu::{BindGroupLayout, Device, Queue, RenderPipeline, Surface, SurfaceConfiguration};
use winit::event::Event;

use crate::node::{Node, NodeLike};

pub struct State<'a> {
    pub physical_size: (u32, u32),
    pub scale_factor: f64,
    pub surface: Arc<Mutex<Surface>>,
    pub device: Arc<Mutex<Device>>,
    pub queue: Arc<Mutex<Queue>>,
    pub config: SurfaceConfiguration,
    pub render_pipeline: Arc<Mutex<RenderPipeline>>,
    pub bind_group_layout: Arc<Mutex<BindGroupLayout>>,
    pub pending_events: Arc<Mutex<Vec<Event<'a, ()>>>>,
    pub pending_updates: Arc<Mutex<Vec<()>>>,
    pub pending_renderable:
        Arc<Mutex<Vec<(wgpu::BindGroup, wgpu::Buffer, wgpu::Buffer, u32, u32)>>>,
    pub root_node: Arc<Mutex<Node>>,
    pub current_focused_node: Arc<Mutex<Option<Arc<Mutex<NodeLike>>>>>,
}

impl<'a> State<'a> {
    pub fn new(
        surface: Arc<Mutex<Surface>>,
        device: Arc<Mutex<Device>>,
        queue: Arc<Mutex<Queue>>,
        config: SurfaceConfiguration,
        render_pipeline: Arc<Mutex<RenderPipeline>>,
        bind_group_layout: Arc<Mutex<BindGroupLayout>>,
    ) -> Self {
        // create root node
        let root_node = Node::new(
            "Root Node".to_string(),
            Default::default(),
            Default::default(),
        );

        Self {
            physical_size: Default::default(),
            scale_factor: Default::default(),
            surface,
            device,
            queue,
            config,
            render_pipeline,
            bind_group_layout,
            pending_events: Default::default(),
            pending_updates: Default::default(),
            pending_renderable: Default::default(),
            root_node: Arc::new(Mutex::new(root_node)),
            current_focused_node: Arc::new(Mutex::new(None)),
        }
    }

    pub fn test(&self) {
        debug!("test!!");
    }

    /**
     * Set screen size before first render, which should not be called after render loop started.
     */
    pub fn set_screen_size(&mut self, physical_size: (u32, u32), scale_factor: f64) {
        self.physical_size = physical_size;
        self.scale_factor = scale_factor;
    }

    /// reset surface
    pub fn refresh(&mut self) {
        self.resize(self.physical_size, None);
    }

    // reconfigure the surface everytime the window's size changes
    pub fn resize(&mut self, new_size: (u32, u32), new_scale_factor: Option<f64>) {
        if new_size.0 > 0 && new_size.1 > 0 {
            // set new physical size
            self.physical_size = new_size;

            // set to surface config as well
            self.config.width = new_size.0;
            self.config.height = new_size.1;

            // dpi may change together
            if let Some(new_scale_factor) = new_scale_factor {
                self.scale_factor = new_scale_factor;
            }

            // apply new size
            let surface = self.surface.lock().unwrap();
            let device = self.device.lock().unwrap();
            surface.configure(&device, &self.config);
        }
    }
}
