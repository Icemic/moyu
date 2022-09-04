use log::error;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard, RwLock},
};
use wgpu::{BindGroupLayout, Device, Queue, RenderPipeline, Surface, SurfaceConfiguration};
use winit::{event::Event, event_loop::EventLoopProxy};

use crate::{
    nodes::Container,
    resource::ResourceManager,
    traits::{Node, Renderable, Renderer},
    user_event::UserEvent,
};

pub struct State<'a> {
    pub physical_size: (u32, u32),
    pub scale_factor: f64,
    pub surface: Arc<Mutex<Surface>>,
    pub device: Arc<Mutex<Device>>,
    pub queue: Arc<Mutex<Queue>>,
    pub config: SurfaceConfiguration,
    pub event_proxy: EventLoopProxy<UserEvent>,
    pub resource_manager: Arc<Mutex<ResourceManager>>,
    renderers: HashMap<String, Box<dyn Renderer>>,

    pub pending_events: Arc<Mutex<Vec<Event<'a, ()>>>>,
    pub pending_updates: Arc<Mutex<Vec<()>>>,
    pub pending_renderable:
        Arc<Mutex<Vec<(wgpu::BindGroup, wgpu::Buffer, wgpu::Buffer, u32, u32)>>>,
    pub root_node: Arc<Mutex<dyn Node>>,
    pub current_focused_node: Arc<Mutex<Option<Arc<Mutex<dyn Node>>>>>,
    pub node_map: Arc<Mutex<HashMap<u32, Arc<Mutex<dyn Node>>>>>,
}

impl<'a> State<'a> {
    pub fn new(
        surface: Arc<Mutex<Surface>>,
        device: Arc<Mutex<Device>>,
        queue: Arc<Mutex<Queue>>,
        config: SurfaceConfiguration,
        event_proxy: EventLoopProxy<UserEvent>,
    ) -> Self {
        // create root node
        let root_node = Container::new(
            "Root Node".to_string(),
            Default::default(),
            Default::default(),
        );
        let root_node = Arc::new(Mutex::new(root_node));

        let mut node_map: HashMap<u32, Arc<Mutex<dyn Node>>> = Default::default();
        node_map.insert(0, root_node.clone());

        let resource_manager = ResourceManager::new(device.clone(), queue.clone());
        let renderers = HashMap::default();

        Self {
            physical_size: Default::default(),
            scale_factor: Default::default(),
            surface,
            device,
            queue,
            config,
            event_proxy,
            resource_manager: Arc::new(Mutex::new(resource_manager)),
            renderers,
            pending_events: Default::default(),
            pending_updates: Default::default(),
            pending_renderable: Default::default(),
            root_node,
            current_focused_node: Arc::new(Mutex::new(None)),
            node_map: Arc::new(Mutex::new(node_map)),
        }
    }

    pub fn resource_manager(&mut self) -> MutexGuard<ResourceManager> {
        self.resource_manager.lock().unwrap()
    }

    pub fn register_renderer(&mut self, name: String, renderer: Box<dyn Renderer>) {
        if self.renderers.contains_key(&name) {
            error!("There's already a renderer named '{}'.", name);
            return;
        }
        self.renderers.insert(name, renderer);
    }

    pub fn get_renderer(&self, name: &str) -> &Box<dyn Renderer> {
        let renderer = self.renderers.get(name);
        renderer.expect(format!("Cannot find a renderer named '{}'", name).as_str())
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
