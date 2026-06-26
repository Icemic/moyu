use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use log::error;
use moyu_pal::config::get_engine_config;
use moyu_pal::platform::show_fatal_error_and_exit;
use moyu_pal::sync::Mutex;
use moyu_pal::time::Instant;
use moyu_resource::ResourceManager;
use wgpu::util::{DeviceExt, StagingBelt};
use wgpu::{COPY_BYTES_PER_ROW_ALIGNMENT, Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

use crate::base::*;
use crate::core::NodeMap;
use crate::core::render_command::RenderCommand;
use crate::core::render_state::RenderState;
use crate::surface::create_wgpu_surface;
use crate::traits::*;
use crate::utils::coordinates::{
    calculate_surface_physical_coordinates,
    calculate_surface_physical_coordinates_by_scale_and_translate,
};
use crate::utils::fps_meter::FpsMeter;
use crate::utils::walk::walk_nodes_enter_leave;

pub struct Graphics {
    pub(crate) window: Arc<Window>,
    pub(crate) instance: Instance,
    pub(crate) surface: Mutex<Option<Surface<'static>>>,
    pub(crate) device: Device,
    pub(crate) queue: Queue,
    pub(crate) config: Arc<Mutex<SurfaceConfiguration>>,

    pub(crate) resource_manager: Arc<ResourceManager>,
    pub(crate) sender: std::sync::mpsc::SyncSender<RenderCommand>,
    pub(crate) receiver: std::sync::mpsc::Receiver<RenderCommand>,

    pub(crate) renderers: Arc<Mutex<HashMap<String, Box<dyn Renderer>>>>,

    node_map: NodeMap,

    staging_belt: RefCell<StagingBelt>,
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
    /// Tuple: (buffer, width, height, bytes_per_row, receiver)
    snapshot_buffer: Arc<
        Mutex<
            Option<(
                wgpu::Buffer,
                u32,
                u32,
                u32,
                Option<std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>>,
            )>,
        >,
    >,
    /// Filter registry for managing filter renderers
    filter_registry: RefCell<crate::core::filter_registry::FilterRegistry>,

    /// Texture pool for filter intermediate textures
    texture_pool: RefCell<crate::core::texture_pool::TexturePool>,
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
        let physical_size = surface_size.physical_size().into();

        let (instance, surface, device, queue, config) =
            create_wgpu_surface(window, &physical_size).await;

        device.on_uncaptured_error(Arc::new(|error| {
            log::error!("wgpu fatal error: {}", error);
            show_fatal_error_and_exit("Fatal graphics error occurred. See log for details.");
        }));

        let renderers = Arc::new(Mutex::new(HashMap::default()));

        let staging_belt = RefCell::new(StagingBelt::new(1024 * 10));

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

        let (sender, receiver) = std::sync::mpsc::sync_channel::<RenderCommand>(1000000);

        let fps_meter = FpsMeter::default();
        let instant = Instant::now();

        // 创建 Filter Registry 并注册滤镜
        let mut filter_registry = crate::core::filter_registry::FilterRegistry::new();
        filter_registry.register(Box::new(
            crate::nodes::filters::BlurPerfectFilterRenderer::new(&device, config.format),
        ));
        filter_registry.register(Box::new(crate::nodes::filters::BlurFilterRenderer::new(
            &device,
            config.format,
        )));
        filter_registry.register(Box::new(
            crate::nodes::filters::ColorAdjustFilterRenderer::new(&device, config.format),
        ));
        let filter_registry = RefCell::new(filter_registry);

        Self {
            window: window.clone(),
            instance,
            surface: Mutex::new(Some(surface)),
            device,
            queue,
            config: Arc::new(Mutex::new(config)),
            resource_manager: Arc::new(resource_manager),
            sender,
            receiver,
            renderers,
            node_map,
            staging_belt,
            mvp_buffer,
            mvp_bind_group,
            fps_meter,
            instant,
            need_reconfigure: AtomicBool::new(false),
            snapshot_requested: AtomicBool::new(false),
            snapshot_buffer: Arc::new(Mutex::new(None)),
            filter_registry,
            texture_pool: RefCell::new(crate::core::texture_pool::TexturePool::new()),
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

    /// Drop the platform surface while keeping device resources alive.
    /// Blocks until the render thread confirms the surface has been released.
    pub fn release_surface(&self) {
        let (done, wait) = std::sync::mpsc::channel();
        if self
            .sender
            .send(RenderCommand::ReleaseSurface { done })
            .is_ok()
        {
            let _ = wait.recv();
        } else {
            self.release_surface_now();
        }
    }

    fn release_surface_now(&self) {
        let surface = self.surface.lock().take();
        drop(surface);
        self.instance.poll_all(true);
    }

    /// Queue a surface recreation on the render thread.
    /// Must be called before the next BeginFrame that follows a release.
    pub fn request_recreate_surface(&self) {
        let _ = self.sender.send(RenderCommand::RecreateSurface);
    }

    /// Recreate the platform surface from the current window (render-thread side).
    fn recreate_surface(&self) {
        let surface = match self.instance.create_surface(self.window.clone()) {
            Ok(s) => s,
            Err(err) => {
                log::error!("Failed to recreate surface: {}", err);
                return;
            }
        };

        let config = self.config.lock().clone();
        surface.configure(&self.device, &config);

        self.surface.lock().replace(surface);
        log::info!("Surface recreated on render thread.");
    }

    pub fn config(&self) -> &Arc<Mutex<SurfaceConfiguration>> {
        &self.config
    }

    pub fn resource_manager(&self) -> &Arc<ResourceManager> {
        &self.resource_manager
    }

    fn surface_can_copy_src(&self) -> bool {
        self.config
            .lock()
            .usage
            .contains(wgpu::TextureUsages::COPY_SRC)
    }

    /// Request a screenshot to be taken on the next render
    pub fn request_snapshot(&self) -> bool {
        if !self.surface_can_copy_src() {
            log::warn!("Snapshot is not supported because the surface texture is not COPY_SRC");
            return false;
        }

        self.snapshot_requested.store(true, Ordering::Relaxed);
        true
    }

    /// Check if there's a screenshot ready and return it.
    ///
    /// Returns: (data, width, height, bytes_per_row, format)
    ///
    /// Note: The returned data may contain padding bytes to align with GPU requirements.
    /// `bytes_per_row` (aka `stride`) indicates the actual number of bytes per row in the buffer,
    /// while the actual image data is `width * 4` bytes per row.
    /// Callers should strip the padding when processing the image.
    pub fn try_get_snapshot(&self) -> Option<(Vec<u8>, u32, u32, u32, wgpu::TextureFormat)> {
        let mut snapshot_buffer = self.snapshot_buffer.lock();
        if let Some((buffer, width, height, bytes_per_row, rx)) = snapshot_buffer.take() {
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
                return Some((
                    rgba_data,
                    width,
                    height,
                    bytes_per_row,
                    self.config.lock().format,
                ));
            } else {
                // Put the buffer back if mapping didn't complete
                *snapshot_buffer = Some((buffer, width, height, bytes_per_row, Some(rx)));
            }
        }
        None
    }

    /// reset surface
    fn refresh(&self) {
        self.instance.poll_all(true);

        let config = self.config.lock().clone();
        if let Some(surface) = self.surface.lock().as_ref() {
            surface.configure(&self.device, &config);
        }

        self.texture_pool.borrow_mut().cleanup(f64::MAX);
    }

    pub fn reconfigure_surface(&self, surface_size: SurfaceSize, stage_size: SurfaceSize) {
        let mut config = self.config.lock();

        let (width, height) = surface_size.physical_size();
        config.width = width;
        config.height = height;

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

    pub fn update(&self) -> Result<()> {
        // fps
        if moyu_pal::config::get_engine_config().show_fps {
            if self.fps_meter.tick() {
                let fps = self.fps_meter.get_fps();
                self.window
                    .set_title(&format!("fps(rendering): {:.1}", fps));
            }
        }

        if self.need_reconfigure.swap(false, Ordering::Relaxed) {
            self.sender.send(RenderCommand::Reconfigure).unwrap();
        }

        let device = self.device.clone();
        let queue = self.queue.clone();

        let config = self.config.lock();
        let surface_width = config.width;
        let surface_height = config.height;

        drop(config);

        let timestamp = self.instant.elapsed().as_secs_f64();

        let scale_factor = self.window.scale_factor() as f32;
        let surface_logical_size = (
            surface_width as f32 / scale_factor,
            surface_height as f32 / scale_factor,
        );

        let stage_logical_size = (
            get_engine_config().stage_size.width() as f32,
            get_engine_config().stage_size.height() as f32,
        );

        self.sender
            .send(RenderCommand::BeginFrame {
                timestamp,
                surface_logical_size,
                stage_logical_size,
                scale_factor,
            })
            .unwrap();

        {
            let root_node = self.node_map.get(&0).unwrap();
            let mut root_node = root_node.upgradable_read();
            let upload_payload = RendererUpdatePayload {
                timestamp,
                resource_manager: self.resource_manager.clone(),
                surface_logical_size,
                stage_logical_size,
                scale_factor,
            };

            let stage_bound = Bound::new(0., 0., stage_logical_size.0, stage_logical_size.1);

            walk_nodes_enter_leave(
                root_node.as_ref(),
                &mut |child, parent| {
                    let mut _child = child.write();
                    _child.pre_update(parent.base());
                    _child.base_mut().update(parent.base(), false);

                    let renderer_type = _child.renderer_type();
                    let mut collect_command = false;

                    if let Some(current_renderer) = self.renderers.lock().get_mut(renderer_type) {
                        current_renderer.update(
                            _child.as_mut(),
                            &device,
                            &queue,
                            &self.sender,
                            &upload_payload,
                        );

                        collect_command =
                            current_renderer.should_collect_commands(_child.as_ref(), &stage_bound);
                        if collect_command {
                            current_renderer.collect_commands(_child.as_ref(), &self.sender);
                        }
                    }

                    collect_command
                },
                &mut |child, _, collect_command| {
                    {
                        let mut _child = child.write();
                        _child.base_mut().calculate_content_bounds();
                    }

                    let _child = child.read();
                    let renderer_type = _child.renderer_type();

                    if let Some(current_renderer) = self.renderers.lock().get(renderer_type) {
                        if collect_command {
                            current_renderer.collect_post_commands(_child.as_ref(), &self.sender);
                        }
                    }
                },
            );

            root_node.with_upgraded(|n| n.base_mut().calculate_content_bounds());
        }

        self.sender
            .send(RenderCommand::EndFrame {
                timestamp: self.instant.elapsed().as_secs_f64(),
            })
            .unwrap();

        Ok(())
    }

    pub fn render(&self, block: bool) -> Result<(), wgpu::SurfaceError> {
        let device = self.device.clone();
        let queue = self.queue.clone();

        let color = &get_engine_config().background_color;
        let color = wgpu::Color {
            r: color.r as f64,
            g: color.g as f64,
            b: color.b as f64,
            a: color.a as f64,
        };

        // Helper function to create render pass
        fn begin_main_render_pass<'a>(
            encoder: &'a mut wgpu::CommandEncoder,
            view: &'a wgpu::TextureView,
            viewport: [f32; 4],
            clear_color: wgpu::Color,
            clear: bool,
        ) -> wgpu::RenderPass<'static> {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: if clear {
                                wgpu::LoadOp::Clear(clear_color)
                            } else {
                                wgpu::LoadOp::Load
                            },
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                })
                .forget_lifetime();

            pass.set_viewport(viewport[0], viewport[1], viewport[2], viewport[3], 0.0, 1.0);

            pass
        }

        let mut staging_belt = self.staging_belt.borrow_mut();

        let mut current_pass: Option<wgpu::RenderPass> = None;

        let mut state = RenderState::default();

        let mut encoder = None;

        let mut belt_encoder = None;

        let mut output = None;

        // TODO: more strict size handling
        let mut surface_logical_size = (1., 1.);
        let mut stage_logical_size = (1., 1.);
        let mut scale_factor = 1.;

        let mut need_skip_current_frame = false;
        // After ReleaseSurface (Android suspend), the surface is gone. Stay in
        // this state and drop every stale frame command until RecreateSurface
        // (sent on resume) rebuilds the surface. Using RecreateSurface as the
        // exit signal — rather than the next BeginFrame — is essential: the
        // update thread keeps producing frames during the suspend/resume gap,
        // so a stale BeginFrame can arrive before resume and must be discarded
        // instead of being rendered against a None surface.
        let mut surface_released = false;

        let mut draw_count = 0;

        loop {
            let mut filter_registry = self.filter_registry.borrow_mut();
            let timestamp = self.instant.elapsed().as_secs_f64();

            let command = {
                if block {
                    if let Ok(command) = self.receiver.recv() {
                        command
                    } else {
                        log::error!("Render command channel unexpectedly disconnected.");
                        break;
                    }
                } else {
                    if let Ok(command) = self.receiver.try_recv() {
                        command
                    } else {
                        break;
                    }
                }
            };

            // After ReleaseSurface, drop stale frame commands until RecreateSurface
            // rebuilds the surface, but allow surface control commands through.
            if surface_released {
                match command {
                    RenderCommand::RecreateSurface => {
                        // Surface is rebuilt; resume normal frame processing.
                        self.recreate_surface();
                        surface_released = false;
                        continue;
                    }
                    RenderCommand::Reconfigure => {
                        // Let Reconfigure run so the surface config is up-to-date.
                        self.refresh();
                        continue;
                    }
                    RenderCommand::ReleaseSurface { done } => {
                        // Another release while already released.
                        self.release_surface_now();
                        let _ = done.send(());
                        continue;
                    }
                    // Drop all stale frame commands (BeginFrame/EndFrame/draw/...)
                    // that arrive before the surface is rebuilt.
                    _ => continue,
                }
            }

            if need_skip_current_frame {
                match command {
                    RenderCommand::EndFrame { .. } => {
                        need_skip_current_frame = false;
                    }
                    // Handle ReleaseSurface even during a skip cycle to avoid
                    // blocking the main thread's Suspended handler indefinitely.
                    RenderCommand::ReleaseSurface { done } => {
                        drop(current_pass.take());
                        output.take();
                        encoder.take();
                        belt_encoder.take();
                        state.clear_frame_resources();

                        self.release_surface_now();

                        need_skip_current_frame = false;
                        surface_released = true;
                        let _ = done.send(());
                    }
                    _ => {
                        // skip other commands
                    }
                }
                continue;
            }

            match command {
                RenderCommand::BeginFrame {
                    timestamp: _,
                    surface_logical_size: surface,
                    stage_logical_size: stage,
                    scale_factor: scale,
                } => {
                    let o = match self
                        .surface
                        .lock()
                        .as_ref()
                        .ok_or(wgpu::SurfaceError::Lost)
                        .and_then(|surface| surface.get_current_texture())
                    {
                        Ok(v) => v,
                        // Reconfigure the surface if lost
                        Err(wgpu::SurfaceError::Lost) => {
                            log::warn!("surface lost, reconfigure.");
                            self.refresh();
                            need_skip_current_frame = true;
                            continue;
                        }
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("surface out of memory, quit.");
                            std::process::exit(1);
                        }
                        Err(wgpu::SurfaceError::Outdated) => {
                            self.refresh();
                            need_skip_current_frame = true;
                            continue;
                        }
                        Err(wgpu::SurfaceError::Timeout) => {
                            log::warn!("surface timeout, ignored.");
                            need_skip_current_frame = true;
                            continue;
                        }
                        Err(wgpu::SurfaceError::Other) => {
                            log::warn!("surface other error, ignored.");
                            need_skip_current_frame = true;
                            continue;
                        }
                    };

                    encoder = Some(device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("Render Command Encoder"),
                        },
                    ));

                    belt_encoder = Some(device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("Belt Command Encoder"),
                        },
                    ));

                    let v = o
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    // Check if the actual texture size matches the expected size derived from update logic
                    // If mismatch, it means the window has been resized but the update thread hasn't caught up,
                    // or the reconfigure command hasn't been processed yet.
                    // in this case, we should skip this frame to avoid texture out-of-bounds errors (e.g. CaptureBackdrop).
                    let expected_width = (surface.0 * scale) as u32;
                    let expected_height = (surface.1 * scale) as u32;

                    if v.texture().width() != expected_width
                        || v.texture().height() != expected_height
                    {
                        log::warn!(
                            "Surface size mismatch, waiting for reconfiguration: \
                            expected=({}, {}), actual=({}, {})",
                            expected_width,
                            expected_height,
                            v.texture().width(),
                            v.texture().height()
                        );
                        need_skip_current_frame = true;
                        continue;
                    }

                    output = Some(o);
                    surface_logical_size = surface;
                    stage_logical_size = stage;
                    scale_factor = scale;

                    state.reset(
                        [0, 0, v.texture().width(), v.texture().height()],
                        surface.0 * scale,
                        surface.1 * scale,
                        v,
                    );

                    // Reset all filter renderers' frame-local state
                    filter_registry.reset_all_frames();
                }
                RenderCommand::EndFrame { timestamp: _ } => {
                    draw_count = 0;

                    drop(current_pass.take());

                    staging_belt.finish();

                    // Handle screenshot request
                    if self.snapshot_requested.swap(false, Ordering::Relaxed) {
                        let config = self.config.lock();
                        let width = config.width;
                        let height = config.height;
                        drop(config);

                        // Calculate aligned bytes_per_row to satisfy COPY_BYTES_PER_ROW_ALIGNMENT (256)
                        let padded_bytes_per_row = (width * 4 + COPY_BYTES_PER_ROW_ALIGNMENT - 1)
                            / COPY_BYTES_PER_ROW_ALIGNMENT
                            * COPY_BYTES_PER_ROW_ALIGNMENT;

                        // Create a buffer to copy the texture data to
                        let buffer_size = (padded_bytes_per_row * height) as u64;
                        let snapshot_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("Screenshot Buffer"),
                            size: buffer_size,
                            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                            mapped_at_creation: false,
                        });

                        // Copy the texture to the buffer
                        encoder.as_mut().unwrap().copy_texture_to_buffer(
                            wgpu::TexelCopyTextureInfo {
                                texture: &output.as_ref().unwrap().texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            wgpu::TexelCopyBufferInfo {
                                buffer: &snapshot_buffer,
                                layout: wgpu::TexelCopyBufferLayout {
                                    offset: 0,
                                    bytes_per_row: Some(padded_bytes_per_row),
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
                        *snapshot_buffer_guard =
                            Some((snapshot_buffer, width, height, padded_bytes_per_row, None));
                    }

                    // TODO: in winit, it is an empty function now, keep an eye on it.
                    self.window.pre_present_notify();

                    queue.submit(
                        std::iter::once(belt_encoder.take().unwrap().finish())
                            .chain(std::iter::once(encoder.take().unwrap().finish())),
                    );
                    staging_belt.recall();

                    state.clear_frame_resources();
                    output.take().unwrap().present();

                    self.texture_pool.borrow_mut().cleanup(timestamp);

                    #[cfg(web)]
                    self.window.request_redraw();
                }
                RenderCommand::WriteBuffer {
                    buffer,
                    offset,
                    data,
                    use_staging_belt,
                } => {
                    if use_staging_belt {
                        staging_belt
                            .write_buffer(
                                &mut belt_encoder.as_mut().unwrap(),
                                &buffer,
                                offset,
                                (data.len() as u64).try_into().unwrap(),
                                &device,
                            )
                            .copy_from_slice(&data);
                    } else {
                        queue.write_buffer(&buffer, offset, &data);
                    }

                    draw_count += 1;

                    if draw_count > 300 {
                        draw_count = 0;

                        staging_belt.finish();

                        queue.submit(std::iter::once(belt_encoder.take().unwrap().finish()));

                        belt_encoder = Some(device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor {
                                label: Some("Belt Command Encoder"),
                            },
                        ));

                        staging_belt.recall();
                    }
                }
                RenderCommand::Draw {
                    pipeline,
                    bind_group,
                    extra_bind_groups,
                    vertex_buffer,
                    index_buffer,
                    instance_buffer,
                    count,
                    instance_count,
                } => {
                    // 确保有活动的 pass
                    let need_create_pass = current_pass.is_none();
                    if need_create_pass {
                        current_pass = Some(begin_main_render_pass(
                            &mut encoder.as_mut().unwrap(),
                            &state.get_current_view().unwrap(),
                            state.get_current_viewport(),
                            color,
                            false,
                        ));
                    }

                    let render_pass = current_pass.as_mut().unwrap();
                    if need_create_pass {
                        // 设置 MVP bind group（只需设置一次）
                        render_pass.set_bind_group(0, &self.mvp_bind_group, &[]);
                    }

                    if let Some(rect) = state.get_current_scissor_rect() {
                        let w = rect[2].max(1);
                        let h = rect[3].max(1);
                        render_pass.set_scissor_rect(rect[0], rect[1], w, h);
                    }

                    render_pass.set_pipeline(&pipeline);
                    render_pass.set_bind_group(1, &bind_group, &[]);
                    for (i, bg) in extra_bind_groups.iter().enumerate() {
                        render_pass.set_bind_group((2 + i) as u32, bg, &[]);
                    }

                    if let Some(vertex_buffer) = vertex_buffer {
                        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    }
                    if let Some(instance_buffer) = instance_buffer {
                        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                    }
                    if let Some(index_buffer) = index_buffer {
                        render_pass
                            .set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                        render_pass.draw_indexed(0..count, 0, 0..instance_count);
                    } else {
                        render_pass.draw(0..count, 0..instance_count);
                    }
                }
                RenderCommand::BeginClip { rect } => {
                    // 确保有活动的 pass
                    if current_pass.is_none() {
                        current_pass = Some(begin_main_render_pass(
                            &mut encoder.as_mut().unwrap(),
                            &state.get_current_view().unwrap(),
                            state.get_current_viewport(),
                            color,
                            false,
                        ));
                        current_pass
                            .as_mut()
                            .unwrap()
                            .set_bind_group(0, &self.mvp_bind_group, &[]);
                    }

                    let render_pass = current_pass.as_mut().unwrap();

                    // 计算捕获区域
                    let (x, y, w, h) = calculate_surface_physical_coordinates(
                        &rect,
                        stage_logical_size,
                        surface_logical_size,
                        scale_factor,
                    );

                    let rect = state.push_scissor_rect(x as i32, y as i32, w as i32, h as i32);
                    render_pass.set_scissor_rect(rect.0, rect.1, rect.2, rect.3);
                }
                RenderCommand::EndClip => {
                    // scissor_stack.pop();
                    state.pop_scissor_rect();
                    if let Some(rect) = state.get_current_scissor_rect() {
                        // 确保有活动的 pass
                        if current_pass.is_none() {
                            current_pass = Some(begin_main_render_pass(
                                &mut encoder.as_mut().unwrap(),
                                &state.get_current_view().unwrap(),
                                state.get_current_viewport(),
                                color,
                                false,
                            ));
                            current_pass.as_mut().unwrap().set_bind_group(
                                0,
                                &self.mvp_bind_group,
                                &[],
                            );
                        }
                        let render_pass = current_pass.as_mut().unwrap();
                        let w = rect[2].max(1);
                        let h = rect[3].max(1);
                        render_pass.set_scissor_rect(rect[0], rect[1], w, h);
                    }
                }
                RenderCommand::Barrier => {
                    // 结束当前 pass 并提交
                    drop(current_pass.take());
                    staging_belt.finish();
                    queue.submit(std::iter::once(belt_encoder.take().unwrap().finish()));
                    queue.submit(std::iter::once(encoder.take().unwrap().finish()));

                    // 创建新的 encoder（暂不开始 pass，等待纹理操作）
                    encoder = Some(device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        },
                    ));

                    belt_encoder = Some(device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("Belt Command Encoder"),
                        },
                    ));
                    staging_belt.recall();
                    current_pass = None;
                }
                RenderCommand::CaptureBackdrop {
                    source_view,
                    final_view,
                    rect: region,
                    filters,
                } => {
                    // 此时 current_pass 应该是 None（刚执行完 Barrier）
                    if current_pass.is_some() {
                        drop(current_pass.take());
                    }

                    // 计算捕获区域

                    let (scale, tx, ty) = get_scale_and_translate(
                        stage_logical_size.0,
                        stage_logical_size.1,
                        surface_logical_size.0,
                        surface_logical_size.1,
                    );

                    let (region_x, region_y, width, height) =
                        calculate_surface_physical_coordinates_by_scale_and_translate(
                            &region,
                            scale,
                            tx,
                            ty,
                            scale_factor,
                        );

                    if width == 0 || height == 0 {
                        continue;
                    }

                    if !self.surface_can_copy_src() {
                        let _clear_pass = encoder.as_mut().unwrap().begin_render_pass(
                            &wgpu::RenderPassDescriptor {
                                label: Some("Clear Backdrop Fallback Pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &final_view,
                                    depth_slice: None,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                ..Default::default()
                            },
                        );
                        log::warn!(
                            "CaptureBackdrop skipped because the surface texture is not COPY_SRC"
                        );
                        continue;
                    }

                    let source_texture = source_view.texture();
                    let final_texture = final_view.texture();

                    if source_texture.width() != width || source_texture.height() != height {
                        log::warn!(
                            "CaptureBackdrop: output texture size ({}, {}) does not match region size ({}, {})",
                            source_texture.width(),
                            source_texture.height(),
                            width,
                            height
                        );
                    }

                    // 2. 复制 output texture 的指定区域到临时纹理
                    encoder.as_mut().unwrap().copy_texture_to_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &output.as_ref().unwrap().texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d {
                                x: region_x,
                                y: region_y,
                                z: 0,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyTextureInfo {
                            texture: &source_texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                    );

                    // 3. 应用滤镜到 final_texture
                    if !filters.is_empty() {
                        filter_registry.execute_filter_chain(
                            &device,
                            &self.queue,
                            &mut encoder.as_mut().unwrap(),
                            &source_view,
                            &final_view,
                            &filters,
                            width,
                            height,
                            scale * scale_factor,
                            state.get_current_format().unwrap(),
                            &mut self.texture_pool.borrow_mut(),
                            timestamp,
                        );
                    } else {
                        // 没有滤镜，直接复制
                        encoder.as_mut().unwrap().copy_texture_to_texture(
                            wgpu::TexelCopyTextureInfo {
                                texture: &source_texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            wgpu::TexelCopyTextureInfo {
                                texture: &final_texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            wgpu::Extent3d {
                                width,
                                height,
                                depth_or_array_layers: 1,
                            },
                        );
                    }
                }
                RenderCommand::Reconfigure => {
                    self.refresh();
                }
                RenderCommand::RecreateSurface => {
                    self.recreate_surface();
                }
                RenderCommand::ReleaseSurface { done } => {
                    drop(current_pass.take());
                    output.take();
                    encoder.take();
                    belt_encoder.take();
                    state.clear_frame_resources();

                    self.release_surface_now();

                    // Drop stale commands from the old frame cycle until
                    // RecreateSurface rebuilds the surface on resume.
                    surface_released = true;
                    let _ = done.send(());
                }
                RenderCommand::BeginRenderTargetPass {
                    target_view,
                    rect,
                    content_origin,
                } => {
                    if let Some(pass) = current_pass.take() {
                        drop(pass);
                    }

                    let (scale, tx, ty) = get_scale_and_translate(
                        stage_logical_size.0,
                        stage_logical_size.1,
                        surface_logical_size.0,
                        surface_logical_size.1,
                    );

                    let w = target_view.texture().width();
                    let h = target_view.texture().height();

                    let content_origin = content_origin.unwrap_or((rect.x(), rect.y()));
                    let origin_x = (content_origin.0 * scale + tx) * scale_factor;
                    let origin_y = (content_origin.1 * scale + ty) * scale_factor;
                    let origin_px = origin_x as u32;
                    let origin_py = origin_y as u32;

                    state.push_offscreen_state(
                        [0, 0, w, h],
                        (origin_px, origin_py),
                        [
                            -origin_x,
                            -origin_y,
                            surface_logical_size.0 * scale_factor,
                            surface_logical_size.1 * scale_factor,
                        ],
                        target_view,
                    );

                    current_pass = Some(begin_main_render_pass(
                        &mut encoder.as_mut().unwrap(),
                        &state.get_current_view().unwrap(),
                        state.get_current_viewport(),
                        wgpu::Color::TRANSPARENT,
                        true,
                    ));

                    current_pass
                        .as_mut()
                        .unwrap()
                        .set_bind_group(0, &self.mvp_bind_group, &[]);
                }
                RenderCommand::EndRenderTargetPass => {
                    if let Some(pass) = current_pass.take() {
                        drop(pass);
                    }

                    state.pop_offscreen_state();

                    current_pass = Some(begin_main_render_pass(
                        &mut encoder.as_mut().unwrap(),
                        &state.get_current_view().unwrap(),
                        state.get_current_viewport(),
                        color,
                        false,
                    ));

                    current_pass
                        .as_mut()
                        .unwrap()
                        .set_bind_group(0, &self.mvp_bind_group, &[]);
                }
                RenderCommand::BeginOffscreenPass {
                    offscreen_view,
                    rect,
                } => {
                    if let Some(pass) = current_pass.take() {
                        drop(pass);
                    }

                    let w = offscreen_view.texture().width();
                    let h = offscreen_view.texture().height();
                    let (scale, tx, ty) = get_scale_and_translate(
                        stage_logical_size.0,
                        stage_logical_size.1,
                        surface_logical_size.0,
                        surface_logical_size.1,
                    );
                    let origin_x = (rect.x() * scale + tx) * scale_factor;
                    let origin_y = (rect.y() * scale + ty) * scale_factor;
                    let origin_px = origin_x as u32;
                    let origin_py = origin_y as u32;

                    state.push_offscreen_state(
                        [0, 0, w, h],
                        (origin_px, origin_py),
                        [
                            -origin_x,
                            -origin_y,
                            surface_logical_size.0 * scale_factor,
                            surface_logical_size.1 * scale_factor,
                        ],
                        offscreen_view.clone(),
                    );

                    // 开始新的 pass（清屏）
                    current_pass = Some(begin_main_render_pass(
                        &mut encoder.as_mut().unwrap(),
                        &state.get_current_view().unwrap(),
                        state.get_current_viewport(),
                        wgpu::Color::TRANSPARENT,
                        true,
                    ));

                    current_pass
                        .as_mut()
                        .unwrap()
                        .set_bind_group(0, &self.mvp_bind_group, &[]);
                }
                RenderCommand::EndOffscreenPass {
                    offscreen_view,
                    final_view,
                    rect,
                    filters,
                } => {
                    if let Some(pass) = current_pass.take() {
                        drop(pass);
                    }

                    state.pop_offscreen_state();

                    let (scale, tx, ty) = get_scale_and_translate(
                        stage_logical_size.0,
                        stage_logical_size.1,
                        surface_logical_size.0,
                        surface_logical_size.1,
                    );

                    let (_, _, w, h) =
                        calculate_surface_physical_coordinates_by_scale_and_translate(
                            &rect,
                            scale,
                            tx,
                            ty,
                            scale_factor,
                        );

                    if !filters.is_empty() {
                        filter_registry.execute_filter_chain(
                            &device,
                            &self.queue,
                            &mut encoder.as_mut().unwrap(),
                            &offscreen_view,
                            &final_view,
                            &filters,
                            w,
                            h,
                            scale * scale_factor,
                            state.get_current_format().unwrap(),
                            &mut self.texture_pool.borrow_mut(),
                            timestamp,
                        );
                    } else {
                        encoder.as_mut().unwrap().copy_texture_to_texture(
                            wgpu::TexelCopyTextureInfo {
                                texture: offscreen_view.texture(),
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            wgpu::TexelCopyTextureInfo {
                                texture: final_view.texture(),
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            wgpu::Extent3d {
                                width: w,
                                height: h,
                                depth_or_array_layers: 1,
                            },
                        );
                    }

                    // 重新开始主 pass
                    current_pass = Some(begin_main_render_pass(
                        &mut encoder.as_mut().unwrap(),
                        &state.get_current_view().unwrap(),
                        state.get_current_viewport(),
                        color,
                        false,
                    ));

                    current_pass
                        .as_mut()
                        .unwrap()
                        .set_bind_group(0, &self.mvp_bind_group, &[]);
                }
            }
        }

        Ok(())
    }
}

impl Drop for Graphics {
    fn drop(&mut self) {
        self.queue.submit(vec![]);
    }
}
