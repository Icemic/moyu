use hai_pal::sync::{Mutex, RwLock};
use log::error;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::forget;
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::event_loop::EventLoopProxy;

use crate::{
    nodes::Container,
    resource::ResourceManager,
    traits::{Node, Renderer},
    types::SurfaceSize,
    user_event::UserEvent,
};

static STATE: OnceCell<usize> = OnceCell::new();

pub fn get_shared_state() -> Arc<State> {
    let p = *STATE.get().unwrap() as *const c_void;
    let ptr = p as *const State;
    let r = unsafe { Arc::from_raw(ptr) };
    let r_cloned = r.clone();

    // keep ptr leaked
    forget(r);

    r_cloned
}

pub fn set_shared_state(state: Arc<State>) {
    let p = Arc::into_raw(state) as *const c_void as usize;
    STATE.set(p).expect("Failed to set shared state.");
}

pub struct State {
    pub surface_size: Arc<Mutex<SurfaceSize>>,
    pub surface: Arc<Surface>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub config: Arc<Mutex<SurfaceConfiguration>>,
    pub event_proxy: Arc<Mutex<EventLoopProxy<UserEvent>>>,
    pub resource_manager: Arc<Mutex<ResourceManager>>,
    pub renderers: Arc<RwLock<HashMap<String, Box<dyn Renderer>>>>,

    pub root_node: Arc<RwLock<dyn Node>>,
    pub current_focused_node: Arc<RwLock<Option<Arc<RwLock<dyn Node>>>>>,
    pub node_map: Arc<RwLock<HashMap<u32, Arc<RwLock<dyn Node>>>>>,
}

impl State {
    pub fn new(
        surface: Arc<Surface>,
        device: Arc<Device>,
        queue: Arc<Queue>,
        config: SurfaceConfiguration,
        event_proxy: Arc<Mutex<EventLoopProxy<UserEvent>>>,
    ) -> Self {
        // create root node
        let root_node = Container::new("Root Node".to_string());
        let root_node = Arc::new(RwLock::new(root_node));

        let mut node_map: HashMap<u32, Arc<RwLock<dyn Node>>> = Default::default();
        node_map.insert(0, root_node.clone());

        let resource_manager = ResourceManager::new(device.clone(), queue.clone());
        let renderers = HashMap::default();

        Self {
            surface_size: Default::default(),
            surface,
            device,
            queue,
            config: Arc::new(Mutex::new(config)),
            event_proxy,
            resource_manager: Arc::new(Mutex::new(resource_manager)),
            renderers: Arc::new(RwLock::new(renderers)),
            root_node,
            current_focused_node: Arc::new(RwLock::new(None)),
            node_map: Arc::new(RwLock::new(node_map)),
        }
    }

    pub fn register_renderer(&self, name: String, renderer: Box<dyn Renderer>) {
        let mut renderers = self.renderers.write();
        if renderers.contains_key(&name) {
            error!("There's already a renderer named '{}'.", name);
            return;
        }
        renderers.insert(name, renderer);
    }

    /**
     * Set screen size before first render, which should not be called after render loop started.
     */
    pub fn set_screen_size(&self, physical_size: (u32, u32), scale_factor: f64) {
        let mut surface_size = self.surface_size.lock();

        surface_size.set_scale_factor(scale_factor);
        surface_size.set_physical_size(physical_size.0, physical_size.1);
    }

    /// reset surface
    pub fn refresh(&self) {
        let config = self.config.lock();
        self.surface.configure(&self.device, &config);
    }

    // reconfigure the surface everytime the window's size changes
    pub fn resize(&self, new_size: SurfaceSize) {
        let (width, height) = new_size.physical_size();

        let mut config = self.config.lock();

        config.width = width;
        config.height = height;

        *(self.surface_size.lock()) = new_size;

        // apply new size
        self.surface.configure(&self.device, &config);
    }
}
