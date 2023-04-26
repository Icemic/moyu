use hai_pal::sync::{Mutex, RwLock, RwLockReadGuard};
use log::{debug, error, info};
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::forget;
use std::sync::Arc;
#[cfg(not(feature = "web"))]
use std::time::Instant;
use wgpu::util::StagingBelt;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::dpi::{LogicalSize, Size};
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopProxy};
use winit::window::Window;

use crate::utils::walk::{walk_nodes_bottom_top, walk_nodes_top_bottom};
use crate::{
    nodes::{Container, Sprite},
    resource::ResourceManager,
    traits::{Focusable, Node, NodeType, Renderer, RendererUpdatePayload},
    types::SurfaceSize,
    user_event::UserEvent,
};

static CORE: OnceCell<usize> = OnceCell::new();

#[inline]
pub fn get_core_optional() -> Option<Arc<Core>> {
    let p = if let Some(p) = CORE.get() {
        *p as *const c_void
    } else {
        return None;
    };

    let ptr = p as *const Core;
    let r = unsafe { Arc::from_raw(ptr) };
    let r_cloned = r.clone();

    // keep ptr leaked
    forget(r);

    Some(r_cloned)
}

#[inline]
pub fn get_core() -> Arc<Core> {
    let p = *CORE.get().unwrap() as *const c_void;
    let ptr = p as *const Core;
    let r = unsafe { Arc::from_raw(ptr) };
    let r_cloned = r.clone();

    // keep ptr leaked
    forget(r);

    r_cloned
}

#[inline]
pub fn set_core(core: Arc<Core>) {
    let p = Arc::into_raw(core) as *const c_void as usize;
    CORE.set(p).expect("Failed to set core instance.");
}

pub struct Core {
    pub surface_size: Arc<RwLock<SurfaceSize>>,
    pub surface: Arc<Surface>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub config: Arc<Mutex<SurfaceConfiguration>>,
    pub event_proxy: Arc<Mutex<EventLoopProxy<UserEvent>>>,
    pub resource_manager: Arc<Mutex<ResourceManager>>,
    pub renderers: Arc<Mutex<HashMap<String, Box<dyn Renderer>>>>,

    staging_belt: Arc<Mutex<StagingBelt>>,
    // std::time not implemented on wasm32 target
    #[cfg(not(feature = "web"))]
    frames_in_duration: Arc<Mutex<(Instant, u32)>>,

    pub root_node: Arc<RwLock<dyn Node>>,
    pub current_focused_node: Arc<RwLock<Option<Arc<RwLock<dyn Node>>>>>,
    pub node_map: Arc<RwLock<HashMap<u32, Arc<RwLock<dyn Node>>>>>,
}

impl Core {
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

        let staging_belt = Arc::new(Mutex::new(StagingBelt::new(0)));

        Self {
            surface_size: Default::default(),
            surface,
            device,
            queue,
            config: Arc::new(Mutex::new(config)),
            event_proxy,
            resource_manager: Arc::new(Mutex::new(resource_manager)),
            renderers: Arc::new(Mutex::new(renderers)),

            staging_belt,
            #[cfg(not(feature = "web"))]
            frames_in_duration: Arc::new(Mutex::new((Instant::now(), 0))),

            root_node,
            current_focused_node: Arc::new(RwLock::new(None)),
            node_map: Arc::new(RwLock::new(node_map)),
        }
    }

    pub fn register_renderer(&self, name: String, renderer: Box<dyn Renderer>) {
        let mut renderers = self.renderers.lock();
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
        let mut surface_size = self.surface_size.write();

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

        *(self.surface_size.write()) = new_size;

        // apply new size
        self.surface.configure(&self.device, &config);
    }

    #[inline(always)]
    pub fn handle_events(
        &self,
        event: Event<UserEvent>,
        window: &Window,
    ) -> (Option<ControlFlow>,) {
        let mut control_flow = None;
        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                match self.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => {
                        self.refresh();
                    }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => control_flow = Some(ControlFlow::Exit),
                    Err(wgpu::SurfaceError::Outdated) => {
                        // ignore
                    }
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                // makes State to have priority over main()
                if !self.input(event) {
                    // UPDATED!
                    match event {
                        WindowEvent::CloseRequested => control_flow = Some(ControlFlow::Exit),
                        WindowEvent::Resized(physical_size) => {
                            let surface_size = SurfaceSize::from_physical_size(
                                physical_size,
                                window.scale_factor(),
                            );

                            if physical_size.width == 0 || physical_size.height == 0 {
                                // window minimized, ignore
                            } else {
                                self.resize(surface_size);
                            }
                        }
                        WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            new_inner_size,
                            ..
                        } => {
                            let surface_size = SurfaceSize::from_physical_size(
                                new_inner_size.to_owned(),
                                scale_factor.clone(),
                            );
                            self.resize(surface_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::UserEvent(user_event) => match user_event {
                UserEvent::ResizeWindow(logical_width, logical_height, factor) => {
                    let factor = factor.unwrap_or(window.scale_factor());

                    if logical_width > 0. && logical_height > 0. {
                        let surface_size = SurfaceSize::new(logical_width, logical_height, factor);
                        self.resize(surface_size);

                        let window_size =
                            Size::Logical(LogicalSize::new(logical_width, logical_height));
                        window.set_inner_size(window_size);
                    }
                }
                UserEvent::SetTitle(title) => {
                    window.set_title(&title);
                }
                UserEvent::Quit => {
                    control_flow = Some(ControlFlow::Exit);
                    info!("Goodbye.");
                }
            },
            _ => {}
        }

        (control_flow,)
    }

    #[inline(always)]
    pub fn render(&self) -> Result<(), wgpu::SurfaceError> {
        // fps
        #[cfg(not(feature = "web"))]
        {
            let (instant, frames) = &mut *self.frames_in_duration.lock();
            let duration = instant.elapsed().as_secs_f32();
            if duration >= 1. {
                let fps = *frames as f32 / duration;

                self.event_proxy
                    .lock()
                    .send_event(UserEvent::SetTitle(format!("fps: {:.1}", fps)))
                    .unwrap();

                *frames = 1;
                *instant = Instant::now();
            } else {
                *frames += 1;
            }
        }

        let surface = self.surface.clone();
        let device = self.device.clone();
        let queue = self.queue.clone();
        let root_node = self.root_node.clone();

        let renderers = self.renderers.clone();
        let mut renderers = renderers.lock();

        let mut staging_belt = self.staging_belt.lock();

        let surface_size = self.surface_size.read();

        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = {
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Command Encoder"),
            })
        };

        let mut belt_encoder = {
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Belt Command Encoder"),
            })
        };

        {
            let root_node = root_node.read();
            let upload_payload = RendererUpdatePayload {
                surface_size: surface_size.clone(),
            };

            let mut nodes: Vec<Arc<RwLock<dyn Node>>> = vec![];

            walk_nodes_top_bottom(&*root_node, &mut |child, parent| {
                let mut _child = child.write();
                _child.update_transform(parent.global_transform(), &surface_size, false);

                let node_type = NodeType::node_type(&*_child);

                if let Some(current_renderer) = renderers.get_mut(node_type) {
                    current_renderer.update(
                        &mut *_child,
                        &device,
                        &queue,
                        &mut belt_encoder,
                        &mut staging_belt,
                        &upload_payload,
                    );
                    drop(_child);
                    nodes.push(child);
                }

                false
            });

            // FIXME: too many loops
            let childs: Vec<RwLockReadGuard<dyn Node>> = nodes.iter().map(|n| n.read()).collect();

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            let childs: Vec<&dyn Node> = childs.iter().map(|n| &**n).collect();

            for child in childs {
                let node_type = NodeType::node_type(child);

                if let Some(current_renderer) = renderers.get(node_type) {
                    current_renderer.render(&device, &queue, &mut render_pass, child);
                }
            }
        }

        staging_belt.finish();

        queue.submit(
            std::iter::once(belt_encoder.finish()).chain(std::iter::once(encoder.finish())),
        );
        output.present();

        staging_belt.recall();

        Ok(())
    }

    #[inline(always)]
    pub fn input(&self, event: &WindowEvent) -> bool {
        let root_node = self.root_node.clone();
        let current_focused_node = self.current_focused_node.clone();

        let surface_size = self.surface_size.read();
        let (logical_width, logical_height) = surface_size.logical_size();
        let scale_factor = surface_size.scale_factor();

        drop(surface_size);

        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let global_logical_x = position.x / scale_factor;
                let global_logical_y = position.y / scale_factor;

                let root_node = root_node.read();

                walk_nodes_bottom_top(&*root_node, &mut |child, parent| {
                    let child_ref = child.read();
                    let hit = match NodeType::node_type(&*child_ref) {
                        "sprite" => {
                            let sprite = child_ref.as_any().downcast_ref::<Sprite>().unwrap();
                            // calculate relative coordinate
                            let parent_global_x = parent.global_transform().tx * logical_width / 2.;
                            let parent_global_y =
                                parent.global_transform().ty * logical_height / 2.;

                            let relative_logical_x = (global_logical_x - parent_global_x).round();
                            let relative_logical_y = (global_logical_y - parent_global_y).round();

                            // check if pointer is over the sprite
                            let hit = sprite.contains(relative_logical_x, relative_logical_y);

                            (hit, Some(sprite.label.clone()))
                        }
                        _ => (false, None),
                    };

                    if hit.0 {
                        let mut current_focused_node = current_focused_node.write();
                        *current_focused_node = Some(child.clone());
                        debug!("pointer is over {}", hit.1.unwrap());
                    }

                    hit.0
                });
                true
            }
            WindowEvent::CursorLeft { .. } => {
                let mut current_focused_node = current_focused_node.write();
                *current_focused_node = None;
                true
            }
            WindowEvent::MouseInput { .. } => {
                //
                debug!("click");
                true
            }
            _ => false,
        }
    }
}
