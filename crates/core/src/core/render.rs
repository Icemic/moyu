use doufu_pal::config::get_engine_config;
use doufu_pal::sync::{Mutex, RwLock};
use doufu_pal::time::Instant;
use log::error;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wgpu::util::{DeviceExt, StagingBelt};
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

use crate::base::*;
use crate::surface::create_wgpu_surface;
use crate::utils::fps_meter::FpsMeter;
use crate::utils::walk::walk_nodes_top_bottom;
use crate::{resource::ResourceManager, traits::*};

pub type AfterRenderHandler = Box<
    dyn Fn(
            &Device,
            &Queue,
            &mut wgpu::CommandEncoder,
            &wgpu::SurfaceTexture,
            &wgpu::TextureView,
            &mut wgpu::util::StagingBelt,
        ) + Send
        + Sync,
>;

pub struct Graphics {
    pub(crate) window: Arc<Window>,
    pub(crate) instance: Arc<Instance>,
    pub(crate) surface: Arc<Surface<'static>>,
    pub(crate) device: Arc<Device>,
    pub(crate) queue: Arc<Queue>,
    pub(crate) config: Arc<Mutex<SurfaceConfiguration>>,

    pub(crate) resource_manager: Arc<ResourceManager>,

    pub(crate) renderers: Arc<Mutex<HashMap<String, Box<dyn Renderer>>>>,

    // render interrupt handler
    pub(crate) after_render_handler: Arc<Mutex<Option<AfterRenderHandler>>>,

    root_node: Arc<RwLock<dyn Node>>,

    staging_belt: Arc<Mutex<StagingBelt>>,
    mvp_buffer: wgpu::Buffer,
    mvp_bind_group: wgpu::BindGroup,
    fps_meter: FpsMeter,
    /// Timer from graphics created.
    ///
    /// Since [Graphics] can be created multiple times, this timer will be reset every time.
    instant: Instant,
    need_reconfigure: AtomicBool,
}

unsafe impl Send for Graphics {}
unsafe impl Sync for Graphics {}

impl Graphics {
    pub fn init(
        window: &Arc<Window>,
        surface_size: &SurfaceSize,
        stage_size: &SurfaceSize,
        root_node: Arc<RwLock<dyn Node>>,
    ) -> Self {
        let (instance, surface, device, queue, config) =
            doufu_pal::task::block_on_without_runtime(create_wgpu_surface(window));

        let renderers = Arc::new(Mutex::new(HashMap::default()));
        let after_render_handler = Arc::new(Mutex::new(None));

        let staging_belt = Arc::new(Mutex::new(StagingBelt::new(0)));

        let surface_logical_size = surface_size.logical_size_f32();
        let stage_logical_size = stage_size.logical_size_f32();

        let mvp_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MVP Buffer"),
            contents: bytemuck::bytes_of(&MVPMatrix::from_logical_size(
                stage_logical_size,
                surface_logical_size,
            )),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let mvp_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MVP Matrix Bind Group"),
            layout: &MVPMatrix::bind_group_layout(&device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: mvp_buffer.as_entire_binding(),
            }],
        });

        let resource_manager = ResourceManager::new(device.clone(), queue.clone());

        let fps_meter = FpsMeter::default();
        let instant = Instant::now();

        Self {
            window: window.clone(),
            instance,
            surface,
            device,
            queue,
            config: Arc::new(Mutex::new(config)),
            resource_manager: Arc::new(resource_manager),
            renderers,
            after_render_handler,
            root_node,
            staging_belt,
            mvp_buffer,
            mvp_bind_group,
            fps_meter,
            instant,
            need_reconfigure: AtomicBool::new(false),
        }
    }

    pub fn register_renderer(&self, name: &str, renderer: Box<dyn Renderer>) {
        let mut renderers = self.renderers.lock();
        if renderers.contains_key(name) {
            error!("There's already a renderer named '{}'.", name);
            return;
        }
        renderers.insert(name.to_owned(), renderer);
    }

    pub fn register_after_render_handler(&self, handler: AfterRenderHandler) {
        let mut after_render_handler = self.after_render_handler.lock();
        *after_render_handler = Some(handler);
    }

    /// Get instance of wgpu. This is useful when you need to do some low-level operations.
    /// However, it may break the encapsulation of the framework, so use it with caution.
    pub fn instance(&self) -> &Arc<Instance> {
        &self.instance
    }

    /// Get device of wgpu. This is useful when you need to do some low-level operations.
    /// However, it may break the encapsulation of the framework, so use it with caution.
    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    /// Get queue of wgpu. This is useful when you need to do some low-level operations.
    /// However, it may break the encapsulation of the framework, so use it with caution.
    pub fn queue(&self) -> &Arc<Queue> {
        &self.queue
    }

    /// Get surface of wgpu. This is useful when you need to do some low-level operations.
    /// However, it may break the encapsulation of the framework, so use it with caution.
    pub fn surface(&self) -> &Arc<Surface<'static>> {
        &self.surface
    }

    pub fn config(&self) -> &Arc<Mutex<SurfaceConfiguration>> {
        &self.config
    }

    /// reset surface
    fn refresh(&self) {
        let config = self.config.lock();
        self.surface.configure(&self.device, &config);
    }

    pub fn reconfigure_surface(&self, surface_size: SurfaceSize, stage_size: SurfaceSize) {
        let mut config = self.config.lock();

        if cfg!(web) {
            // on web, we need to set physical size to logical size
            // wtf, not sure why this is needed, but it works.
            let (width, height) = surface_size.logical_size();
            config.width = width.round() as u32;
            config.height = height.round() as u32;
        } else {
            let (width, height) = surface_size.physical_size();
            config.width = width;
            config.height = height;
        }

        let stage_size = stage_size.logical_size_f32();

        self.queue.write_buffer(
            &self.mvp_buffer,
            0,
            bytemuck::bytes_of(&MVPMatrix::from_logical_size(
                stage_size,
                surface_size.logical_size_f32(),
            )),
        );

        self.need_reconfigure.store(true, Ordering::Relaxed);
    }

    /// force clear render queue in case of unexpected error (for example, memory leak).
    pub fn clear_queue(&self) {
        self.queue.submit(vec![]);
    }

    pub fn render(
        &self,
        // root_node: &Arc<RwLock<dyn Node>>,
        // resource_manager: &Arc<ResourceManager>,
    ) -> Result<(), wgpu::SurfaceError> {
        // fps
        if doufu_pal::config::get_engine_config().show_fps {
            if self.fps_meter.tick() {
                let fps = self.fps_meter.get_fps();
                self.window
                    .set_title(&format!("fps(rendering): {:.1}", fps));
            }
        }

        if self.need_reconfigure.swap(false, Ordering::Relaxed) {
            let config = self.config.lock();
            // Finish all queue commands before reconfigure.
            // This is essential on DirectX 12 backend to avoid unexpected error.
            self.instance.poll_all(true);
            // apply new size
            self.surface.configure(&self.device, &config);
        }

        let surface = self.surface.clone();
        let device = self.device.clone();
        let queue = self.queue.clone();

        let mut staging_belt = self.staging_belt.lock();

        let output = match surface.get_current_texture() {
            Ok(v) => v,
            // Reconfigure the surface if lost
            Err(wgpu::SurfaceError::Lost) => {
                log::warn!("surface lost, reconfigure.");
                self.refresh();
                return Ok(());
            }
            // The system is out of memory, we should probably quit
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("surface out of memory, quit.");
                std::process::exit(1);
            }
            Err(wgpu::SurfaceError::Outdated) => {
                log::warn!("surface outdated, reconfigure.");
                self.refresh();
                return Err(wgpu::SurfaceError::Outdated);
            }
            Err(wgpu::SurfaceError::Timeout) => {
                log::warn!("surface timeout, ignored.");
                return Ok(());
            }
        };

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
            let root_node = self.root_node.read();

            let timestamp = self.instant.elapsed().as_secs_f64();

            let upload_payload = RendererUpdatePayload {
                timestamp,
                resource_manager: self.resource_manager.clone(),
            };

            let color = &get_engine_config().background_color;
            let color = wgpu::Color {
                r: color.r,
                g: color.g,
                b: color.b,
                a: color.a,
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            render_pass.set_bind_group(0, &self.mvp_bind_group, &[]);

            walk_nodes_top_bottom(&*root_node, &mut |child, parent| {
                let mut _child = child.write();
                _child.base_mut().update(parent.base(), false);

                let renderer_type = _child.renderer_type();

                if let Some(current_renderer) = self.renderers.lock().get_mut(renderer_type) {
                    current_renderer.update(
                        &mut *_child,
                        &device,
                        &queue,
                        &mut belt_encoder,
                        &mut staging_belt,
                        &upload_payload,
                    );

                    current_renderer.render(&device, &queue, &mut render_pass, &*_child);
                }

                false
            });
        }

        // call after render callback if registered
        if let Some(after_render_callback) = self.after_render_handler.lock().as_ref() {
            after_render_callback(
                &device,
                &queue,
                &mut encoder,
                &output,
                &view,
                &mut staging_belt,
            );
        }

        staging_belt.finish();

        // TODO: in winit, it is an empty function now, keep an eye on it.
        self.window.pre_present_notify();

        queue.submit(
            std::iter::once(belt_encoder.finish()).chain(std::iter::once(encoder.finish())),
        );
        output.present();

        staging_belt.recall();

        self.window.request_redraw();

        Ok(())
    }
}

impl Drop for Graphics {
    fn drop(&mut self) {
        self.queue.submit(vec![]);
    }
}
