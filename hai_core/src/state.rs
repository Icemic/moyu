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
    user_event::UserEvent,
};

static STATE: OnceCell<usize> = OnceCell::new();

pub fn get_shared_state() -> Arc<RwLock<State>> {
    let p = *STATE.get().unwrap() as *const c_void;
    let ptr = p as *const RwLock<State>;
    let r = unsafe { Arc::from_raw(ptr) };
    let r_cloned = r.clone();

    // keep ptr leaked
    forget(r);

    r_cloned
}

pub fn set_shared_state(state: Arc<RwLock<State>>) {
    let p = Arc::into_raw(state) as *const c_void as usize;
    STATE.set(p).expect("Failed to set shared state.");
}

pub struct State {
    pub physical_size: (u32, u32),
    pub scale_factor: f64,
    pub surface: Arc<Mutex<Surface>>,
    pub device: Arc<Mutex<Device>>,
    pub queue: Arc<Mutex<Queue>>,
    pub config: SurfaceConfiguration,
    pub event_proxy: Arc<Mutex<EventLoopProxy<UserEvent>>>,
    pub resource_manager: Arc<Mutex<ResourceManager>>,
    pub renderers: Arc<RwLock<HashMap<String, Box<dyn Renderer>>>>,

    pub root_node: Arc<RwLock<dyn Node>>,
    pub current_focused_node: Arc<RwLock<Option<Arc<RwLock<dyn Node>>>>>,
    pub node_map: Arc<RwLock<HashMap<u32, Arc<RwLock<dyn Node>>>>>,
}

impl State {
    pub fn new(
        surface: Arc<Mutex<Surface>>,
        device: Arc<Mutex<Device>>,
        queue: Arc<Mutex<Queue>>,
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
            physical_size: Default::default(),
            scale_factor: Default::default(),
            surface,
            device,
            queue,
            config,
            event_proxy,
            resource_manager: Arc::new(Mutex::new(resource_manager)),
            renderers: Arc::new(RwLock::new(renderers)),
            root_node,
            current_focused_node: Arc::new(RwLock::new(None)),
            node_map: Arc::new(RwLock::new(node_map)),
        }
    }

    pub fn register_renderer(&mut self, name: String, renderer: Box<dyn Renderer>) {
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
            let surface = self.surface.lock();
            let device = self.device.lock();
            surface.configure(&device, &self.config);
        }
    }
}
