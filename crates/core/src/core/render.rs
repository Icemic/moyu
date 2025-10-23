use log::error;
use moyu_pal::config::get_engine_config;
use moyu_pal::sync::Mutex;
use moyu_pal::time::Instant;
use moyu_resource::ResourceManager;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use wgpu::util::{DeviceExt, StagingBelt};
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

use crate::base::*;
use crate::core::NodeMap;
use crate::surface::create_wgpu_surface;
use crate::traits::*;
use crate::utils::fps_meter::FpsMeter;
use crate::utils::walk::walk_nodes_top_bottom;

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
    pub(crate) instance: Instance,
    pub(crate) surface: Surface<'static>,
    pub(crate) device: Device,
    pub(crate) queue: Queue,
    pub(crate) config: Arc<Mutex<SurfaceConfiguration>>,

    pub(crate) resource_manager: Arc<ResourceManager>,

    pub(crate) renderers: Arc<Mutex<HashMap<String, Box<dyn Renderer>>>>,

    // render interrupt handler
    pub(crate) after_render_handler: Arc<Mutex<Option<AfterRenderHandler>>>,

    node_map: NodeMap,

    staging_belt: Arc<Mutex<StagingBelt>>,
    mvp_buffer: wgpu::Buffer,
    mvp_bind_group: wgpu::BindGroup,
    fps_meter: FpsMeter,
    /// Timer from graphics created.
    ///
    /// Since [Graphics] can be created multiple times, this timer will be reset every time.
    instant: Instant,
    need_reconfigure: AtomicBool,
    /// Flag to request a screenshot on the next render
    snapshot_requested: AtomicBool,
    /// Buffer to store screenshot data
    snapshot_buffer: Arc<
        Mutex<
            Option<(
                wgpu::Buffer,
                u32,
                u32,
                Option<std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>>,
            )>,
        >,
    >,
}

unsafe impl Send for Graphics {}
unsafe impl Sync for Graphics {}

impl Graphics {
    pub async fn init(
        window: &Arc<Window>,
        surface_size: &SurfaceSize,
        stage_size: &SurfaceSize,
        node_map: NodeMap,
    ) -> Self {
        let (instance, surface, device, queue, config) = create_wgpu_surface(window).await;

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
            node_map,
            staging_belt,
            mvp_buffer,
            mvp_bind_group,
            fps_meter,
            instant,
            need_reconfigure: AtomicBool::new(false),
            snapshot_requested: AtomicBool::new(false),
            snapshot_buffer: Arc::new(Mutex::new(None)),
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
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    /// Get device of wgpu. This is useful when you need to do some low-level operations.
    /// However, it may break the encapsulation of the framework, so use it with caution.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get queue of wgpu. This is useful when you need to do some low-level operations.
    /// However, it may break the encapsulation of the framework, so use it with caution.
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Get surface of wgpu. This is useful when you need to do some low-level operations.
    /// However, it may break the encapsulation of the framework, so use it with caution.
    pub fn surface(&self) -> &Surface<'static> {
        &self.surface
    }

    pub fn config(&self) -> &Arc<Mutex<SurfaceConfiguration>> {
        &self.config
    }

    pub fn resource_manager(&self) -> &Arc<ResourceManager> {
        &self.resource_manager
    }

    /// Request a screenshot to be taken on the next render
    pub fn request_snapshot(&self) {
        self.snapshot_requested.store(true, Ordering::Relaxed);
    }

    /// Check if there's a screenshot ready and return it
    pub fn try_get_snapshot(&self) -> Option<(Vec<u8>, u32, u32, wgpu::TextureFormat)> {
        let mut snapshot_buffer = self.snapshot_buffer.lock();
        if let Some((buffer, width, height, rx)) = snapshot_buffer.take() {
            let buffer_slice = buffer.slice(..);

            // If there's no receiver, we need to start mapping
            let rx = if rx.is_none() {
                // Try to map the buffer asynchronously
                let (tx, rx) = std::sync::mpsc::channel();
                buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                    let _ = tx.send(result);
                });
                if let Err(err) = self.device.poll(wgpu::PollType::wait_indefinitely()) {
                    log::error!("Failed to poll device for snapshot: {}", err);
                    return None;
                }
                rx
            } else {
                rx.unwrap()
            };

            // Check if mapping completed
            if let Ok(Ok(())) = rx.try_recv() {
                let data = buffer_slice.get_mapped_range();
                let rgba_data = data.to_vec();
                drop(data);
                buffer.unmap();
                return Some((rgba_data, width, height, self.config.lock().format));
            } else {
                // Put the buffer back if mapping didn't complete
                *snapshot_buffer = Some((buffer, width, height, Some(rx)));
            }
        }
        None
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
        // root_node: &NodeLock,
        // resource_manager: &Arc<ResourceManager>,
    ) -> Result<(), wgpu::SurfaceError> {
        // fps
        if moyu_pal::config::get_engine_config().show_fps {
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

        let device = self.device.clone();
        let queue = self.queue.clone();

        let mut staging_belt = self.staging_belt.lock();

        let output = match self.surface.get_current_texture() {
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
                self.refresh();
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => {
                log::warn!("surface timeout, ignored.");
                return Ok(());
            }
            Err(wgpu::SurfaceError::Other) => {
                log::warn!("surface other error, ignored.");
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
            Some(
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Belt Command Encoder"),
                }),
            )
        };

        {
            let root_node = self.node_map.get(&0).unwrap();
            let root_node = root_node.read();

            let timestamp = self.instant.elapsed().as_secs_f64();

            let upload_payload = RendererUpdatePayload {
                timestamp,
                resource_manager: self.resource_manager.clone(),
            };

            let color = &get_engine_config().background_color;
            let color = wgpu::Color {
                r: color.r as f64,
                g: color.g as f64,
                b: color.b as f64,
                a: color.a as f64,
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
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

            let mut count = 0;

            walk_nodes_top_bottom(&*root_node, &mut |child, parent| {
                let mut _child = child.write();
                _child.base_mut().update(parent.base(), false);

                let renderer_type = _child.renderer_type();

                if let Some(current_renderer) = self.renderers.lock().get_mut(renderer_type) {
                    current_renderer.update(
                        &mut *_child,
                        &device,
                        &queue,
                        belt_encoder.as_mut().unwrap(),
                        &mut staging_belt,
                        &upload_payload,
                    );

                    current_renderer.render(&device, &queue, &mut render_pass, &*_child);
                }

                count += 1;

                if count > 100 {
                    count = 0;

                    staging_belt.finish();

                    queue.submit(std::iter::once(belt_encoder.take().unwrap().finish()));

                    belt_encoder = Some(device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("Belt Command Encoder"),
                        },
                    ));

                    staging_belt.recall();
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

        // Handle screenshot request
        if self.snapshot_requested.swap(false, Ordering::Relaxed) {
            let config = self.config.lock();
            let width = config.width;
            let height = config.height;
            drop(config);

            // Create a buffer to copy the texture data to
            let buffer_size = (width * height * 4) as u64; // RGBA
            let snapshot_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Screenshot Buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            // Copy the texture to the buffer
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &output.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &snapshot_buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * width),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );

            // Store the buffer for later retrieval
            let mut snapshot_buffer_guard = self.snapshot_buffer.lock();
            *snapshot_buffer_guard = Some((snapshot_buffer, width, height, None));
        }

        // TODO: in winit, it is an empty function now, keep an eye on it.
        self.window.pre_present_notify();

        queue.submit(
            std::iter::once(belt_encoder.take().unwrap().finish())
                .chain(std::iter::once(encoder.finish())),
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
