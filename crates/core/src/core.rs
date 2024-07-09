use arc_swap::{ArcSwap, ArcSwapOption};
use hai_pal::sync::{Mutex, RwLock, RwLockReadGuard};
use hai_pal::time::Instant;
use hai_pal::visible_hand::{InvisibleHand, VisibleHand};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use wgpu::util::{DeviceExt, StagingBelt};
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::dpi::{LogicalSize, PhysicalPosition, Size};
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use winit::window::{CursorIcon, Fullscreen, Window};

use crate::base::*;
use crate::user_event::WindowState;
use crate::utils::dispatch_event::{dispatch_event, HaiEvent, HaiEventKind};
use crate::utils::hit_test::{hit_test, HitTestResult};
use crate::utils::walk::walk_nodes_top_bottom;
use crate::{nodes::Container, resource::ResourceManager, traits::*, user_event::UserEvent};

static mut CORE: InvisibleHand<Arc<Core>> = InvisibleHand::new();

#[inline]
pub fn get_core<'a>() -> &'a Arc<Core> {
    unsafe { CORE.get() }
}

#[inline]
pub fn set_core(core: Arc<Core>) -> VisibleHand<Arc<Core>> {
    unsafe {
        CORE.set(core).expect("Failed to set core instance.");
        CORE.intervent()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaiRedrawMode {
    /// redraws every frame
    Auto,
    /// only redraws when `Core::is_dirty` is `true, then
    /// set `Core::is_dirty` to `false` after re-drawing.
    Dirty,
}

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

pub struct Core {
    pub(crate) config: Arc<Mutex<SurfaceConfiguration>>,
    pub(crate) event_proxy: Arc<EventLoopProxy<UserEvent>>,
    pub(crate) resource_manager: Arc<ResourceManager>,
    pub(crate) renderers: Arc<Mutex<HashMap<String, Box<dyn Renderer>>>>,

    plugins: Arc<Mutex<HashMap<String, Arc<Mutex<dyn Plugin>>>>>,

    staging_belt: Arc<Mutex<StagingBelt>>,
    mvp_buffer: wgpu::Buffer,
    mvp_bind_group: wgpu::BindGroup,
    // std::time not implemented on wasm32 target
    #[cfg(not(feature = "web"))]
    frames_in_duration: Arc<Mutex<(Instant, u32)>>,
    /// timer from program start
    instant: Instant,
    /// time elapsed since last frame, in microseconds
    instant_last: ArcSwap<Instant>,

    pub(crate) root_node: Arc<RwLock<dyn Node>>,
    pub(crate) last_focused_position: ArcSwapOption<PhysicalPosition<f64>>,
    pub(crate) last_focused_node: Arc<RwLock<Option<HitTestResult>>>,
    pub(crate) mouse_down_id: AtomicU32,
    pub(crate) node_map: Arc<RwLock<HashMap<u32, Arc<RwLock<dyn Node>>>>>,

    pub(crate) window_state: ArcSwap<WindowState>,

    /// redraw mode, default is `Auto`
    pub(crate) redraw_mode: ArcSwap<HaiRedrawMode>,
    /// if `true`, the screen will be refreshed in next frame,
    /// by default it will be `true` to render every frame.
    pub(crate) is_dirty: AtomicBool,

    // render interrupt handler
    pub(crate) after_render_handler: Arc<Mutex<Option<AfterRenderHandler>>>,

    // To avoid memory leak, we must put these at bottom to make sure [Device] is dropped last.
    // see: https://github.com/gfx-rs/wgpu/issues/5529
    pub(crate) queue: Arc<Queue>,
    // To avoid STATUS_ACCESS_VIOLATION on quit, [Surface] must not be the last one to drop, wth...
    // see: https://github.com/gfx-rs/wgpu/issues/5637
    pub(crate) surface: Arc<Surface<'static>>,
    pub(crate) device: Arc<Device>,
    /// Size of current surface, which means the size of the window on desktop platforms, the size of the canvas \
    /// on web platform, and the size of the screen on mobile platforms.
    pub(crate) surface_size: Arc<RwLock<SurfaceSize>>,
    /// Size of stage, which is the content size set by user.
    pub(crate) stage_size: Arc<RwLock<SurfaceSize>>,
    pub(crate) window: Arc<Window>,
    pub(crate) instance: Arc<Instance>,
}

impl Drop for Core {
    fn drop(&mut self) {
        self.queue.submit(vec![]);
    }
}

unsafe impl Send for Core {}
unsafe impl Sync for Core {}

impl Core {
    pub fn new(
        instance: Arc<Instance>,
        surface: Arc<Surface<'static>>,
        device: Arc<Device>,
        queue: Arc<Queue>,
        window: Arc<Window>,
        config: SurfaceConfiguration,
        event_proxy: Arc<EventLoopProxy<UserEvent>>,
    ) -> Self {
        // create root node
        let root_node = Container::new("Root Node".to_string());
        let root_node = Arc::new(RwLock::new(root_node));

        let mut node_map: HashMap<u32, Arc<RwLock<dyn Node>>> = Default::default();
        node_map.insert(0, root_node.clone());

        let resource_manager = ResourceManager::new(device.clone(), queue.clone());
        let renderers = HashMap::default();

        let stage_size = SurfaceSize::default();

        let staging_belt = Arc::new(Mutex::new(StagingBelt::new(0)));

        let logical_size = stage_size.logical_size_f32();
        let mvp_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MVP Buffer"),
            contents: bytemuck::bytes_of(&MVPMatrix::from_logical_size(logical_size)),
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

        Self {
            instance,
            surface_size: Arc::new(RwLock::new(stage_size)),
            stage_size: Arc::new(RwLock::new(stage_size)),
            surface,
            device,
            queue,
            window,
            config: Arc::new(Mutex::new(config)),
            event_proxy,
            resource_manager: Arc::new(resource_manager),
            renderers: Arc::new(Mutex::new(renderers)),

            plugins: Arc::new(Mutex::new(HashMap::new())),

            staging_belt,
            mvp_buffer,
            mvp_bind_group,
            #[cfg(not(feature = "web"))]
            frames_in_duration: Arc::new(Mutex::new((Instant::now(), 0))),
            instant: Instant::now(),
            instant_last: ArcSwap::new(Arc::new(Instant::now())),

            root_node,
            last_focused_position: ArcSwapOption::new(None),
            last_focused_node: Arc::new(RwLock::new(None)),
            mouse_down_id: AtomicU32::new(0),
            node_map: Arc::new(RwLock::new(node_map)),

            window_state: ArcSwap::new(Arc::new(WindowState::Idle)),
            redraw_mode: ArcSwap::new(Arc::new(HaiRedrawMode::Auto)),
            is_dirty: AtomicBool::new(true),

            after_render_handler: Arc::new(Mutex::new(None)),
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

    pub fn register_after_render_handler(&self, handler: AfterRenderHandler) {
        let mut after_render_handler = self.after_render_handler.lock();
        *after_render_handler = Some(handler);
    }

    pub fn register_plugin(&self, name: String, plugin: Arc<Mutex<dyn Plugin>>) {
        let mut plugins = self.plugins.lock();
        if plugins.contains_key(&name) {
            error!("There's already a plugin named '{}'.", name);
            return;
        }
        plugins.insert(name, plugin);
    }

    pub fn get_plugin(&self, name: &str) -> Option<Arc<Mutex<dyn Plugin>>> {
        let plugins = self.plugins.lock();
        plugins.get(name).cloned()
    }

    /**
     * Set screen size before first render, which should not be called after render loop started.
     */
    pub fn set_screen_size(&self, physical_size: (u32, u32), scale_factor: f64) {
        let mut stage_size = self.stage_size.write();

        stage_size.set_scale_factor(scale_factor);
        stage_size.set_physical_size(physical_size.0, physical_size.1);

        self.queue.write_buffer(
            &self.mvp_buffer,
            0,
            bytemuck::bytes_of(&MVPMatrix::from_logical_size(stage_size.logical_size_f32())),
        );
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

    /// reset surface
    pub fn refresh(&self) {
        let config = self.config.lock();
        self.surface.configure(&self.device, &config);
    }

    pub fn set_redraw_mode(&self, mode: HaiRedrawMode) {
        self.redraw_mode.store(Arc::new(mode));
    }

    pub fn set_dirty(&self, is_dirty: bool) {
        self.is_dirty.store(is_dirty, Ordering::Relaxed);
    }

    /// resize window, should be called in main thread
    pub fn resize_window(&self, logical_width: f64, logical_height: f64, factor: Option<f64>) {
        let window = &self.window;
        let factor = factor.unwrap_or(window.scale_factor());

        let window_fullscreen = window.fullscreen();
        let window_minimized = window.is_minimized();
        let window_maximized = window.is_maximized();

        if logical_width > 0. && logical_height > 0. {
            let stage_size = SurfaceSize::new(logical_width, logical_height, factor);
            self.resize_surface(stage_size);

            let window_size = Size::Logical(LogicalSize::new(logical_width, logical_height));

            let _ = window.request_inner_size(window_size);

            window.set_minimized(window_minimized.unwrap_or(false));
            window.set_maximized(window_maximized);

            // reset fullscreen status
            window.set_fullscreen(None);
            window.set_fullscreen(window_fullscreen);
        }
    }

    // reconfigure the surface everytime the window's size changes
    pub fn resize_surface(&self, new_size: SurfaceSize) {
        let mut config = self.config.lock();

        if cfg!(feature = "web") {
            // on web, we need to set physical size to logical size
            // wtf, not sure why this is needed, but it works.
            let (width, height) = new_size.logical_size();
            config.width = width.round() as u32;
            config.height = height.round() as u32;
        } else {
            let (width, height) = new_size.physical_size();
            config.width = width;
            config.height = height;
        }

        *(self.stage_size.write()) = new_size;

        self.queue.write_buffer(
            &self.mvp_buffer,
            0,
            bytemuck::bytes_of(&MVPMatrix::from_logical_size(new_size.logical_size_f32())),
        );

        // Finish all queue commands before reconfigure.
        // This is essential on DirectX 12 backend to avoid unexpected error.
        self.instance.poll_all(true);
        // apply new size
        self.surface.configure(&self.device, &config);
    }

    /// get whole node map
    pub fn node_map(&self) -> &Arc<RwLock<HashMap<u32, Arc<RwLock<dyn Node>>>>> {
        &self.node_map
    }

    /// get root node
    pub fn root_node(&self) -> &Arc<RwLock<dyn Node>> {
        &self.root_node
    }

    pub fn resource_manager(&self) -> &Arc<ResourceManager> {
        &self.resource_manager
    }

    /// dispatch user event
    pub fn send_event(&self, event: UserEvent) {
        if let Err(err) = self.event_proxy.send_event(event) {
            error!("Failed to send event: {:?}", err);
        }
    }

    /// get current surface size
    pub fn stage_size(&self) -> SurfaceSize {
        self.stage_size.read().clone()
    }

    pub fn fullscreen(&self) -> bool {
        let window_state = self.window_state.load();
        match **window_state {
            WindowState::Fullscreen => true,
            _ => false,
        }
    }

    pub fn maximized(&self) -> bool {
        let window_state = self.window_state.load();
        match **window_state {
            WindowState::Maximized => true,
            _ => false,
        }
    }

    pub fn minimized(&self) -> bool {
        let window_state = self.window_state.load();
        match **window_state {
            WindowState::Minimized => true,
            _ => false,
        }
    }

    pub fn idle(&self) -> bool {
        let window_state = self.window_state.load();
        match **window_state {
            WindowState::Idle => true,
            _ => false,
        }
    }

    /// set window state, should be called in main thread
    pub fn set_window_state(&self, state: WindowState) {
        let window = &self.window;
        // get current focus state since focus may lost after state changes.
        let has_focus = window.has_focus();

        match state {
            WindowState::Idle => {
                window.set_maximized(false);
                window.set_minimized(false);
                window.set_fullscreen(None);
            }
            WindowState::Maximized => {
                window.set_fullscreen(None);
                window.set_maximized(true);
            }
            WindowState::Minimized => {
                window.set_fullscreen(None);
                window.set_minimized(true);
            }
            WindowState::Fullscreen => {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        };

        // restore focus state
        if has_focus && state != WindowState::Minimized {
            window.focus_window();
        }

        self.window_state.store(Arc::new(state));
    }

    pub fn set_fullscreen_with_monitor(&self, monitor: Option<winit::monitor::MonitorHandle>) {
        self.window
            .set_fullscreen(Some(Fullscreen::Borderless(monitor)));
        self.window_state.store(Arc::new(WindowState::Fullscreen));
    }

    /// force clear render queue in case of unexpected error (for example, memory leak).
    pub fn clear_queue(&self) {
        self.queue.submit(vec![]);
    }

    #[inline(always)]
    pub fn handle_events(
        &self,
        event: &Event<UserEvent>,
        window: &Window,
        event_loop: &EventLoopWindowTarget<UserEvent>,
    ) {
        match event {
            &Event::AboutToWait => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                let redraw_mode = self.redraw_mode.load();
                match **redraw_mode {
                    HaiRedrawMode::Auto => {
                        window.request_redraw();
                    }
                    HaiRedrawMode::Dirty => {
                        // skip rendering if not dirty
                        if self.is_dirty.load(Ordering::Relaxed) {
                            self.set_dirty(false);
                            window.request_redraw();
                        }
                    }
                }
            }
            &Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                // makes State to have priority over main()
                if !self.input(event) {
                    // UPDATED!
                    match event {
                        WindowEvent::RedrawRequested => {
                            match self.render(window) {
                                Ok(_) => {}
                                // Reconfigure the surface if lost
                                Err(wgpu::SurfaceError::Lost) => {
                                    warn!("surface lost, reconfigure.");
                                    self.refresh();
                                }
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    error!("surface out of memory, quit.");
                                    event_loop.exit();
                                }
                                Err(wgpu::SurfaceError::Outdated) => {
                                    // ignore
                                    warn!("surface outdated, ignored.");
                                }
                                // All other errors (Outdated, Timeout) should be resolved by the next frame
                                Err(e) => {
                                    error!("surface error: {:?}", e);
                                }
                            }
                        }
                        WindowEvent::CloseRequested => event_loop.exit(),
                        WindowEvent::Resized(physical_size) => {
                            let stage_size = SurfaceSize::from_physical_size(
                                physical_size,
                                window.scale_factor(),
                            );

                            if physical_size.width == 0 || physical_size.height == 0 {
                                // window minimized, ignore
                            } else {
                                self.resize_surface(stage_size);
                            }

                            if window.fullscreen().is_some() {
                                self.window_state.store(Arc::new(WindowState::Fullscreen));
                            } else if window.is_maximized() {
                                self.window_state.store(Arc::new(WindowState::Maximized));
                            } else if let Some(true) = window.is_minimized() {
                                self.window_state.store(Arc::new(WindowState::Minimized));
                            } else {
                                self.window_state.store(Arc::new(WindowState::Idle));
                            }

                            debug!("window state changes to {:?}", self.window_state.load());
                        }
                        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                            self.stage_size.write().set_scale_factor(*scale_factor);
                        }
                        _ => {}
                    }
                }
            }
            Event::UserEvent(user_event) => match user_event {
                &UserEvent::ResizeWindow(logical_width, logical_height, factor) => {
                    self.resize_window(logical_width, logical_height, factor);
                }
                &UserEvent::WindowState(state) => {
                    self.set_window_state(state);
                }
                UserEvent::SetTitle(ref title) => {
                    window.set_title(title);
                }
                &UserEvent::SetCursorIcon(icon) => {
                    window.set_cursor_icon(icon);
                }
                &UserEvent::SetCursorVisible(visible) => {
                    window.set_cursor_visible(visible);
                }
                UserEvent::Quit => {
                    info!("Goodbye.");
                    event_loop.exit();
                }
                UserEvent::Custom(_) => {
                    // do nothing
                }
            },
            _ => {}
        }
    }

    #[inline(always)]
    pub fn render(&self, window: &Window) -> Result<(), wgpu::SurfaceError> {
        // fps
        #[cfg(not(feature = "web"))]
        if hai_pal::env::get_hai_env().show_fps {
            let (instant, frames) = &mut *self.frames_in_duration.lock();
            let duration = instant.elapsed().as_secs_f32();
            if duration >= 1. {
                let fps = *frames as f32 / duration;

                self.event_proxy
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
        let stage_size = self.stage_size.read();

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

            let timestamp = self.instant.elapsed().as_secs_f64();
            let instant_last = self.instant_last.swap(Arc::new(Instant::now()));

            let upload_payload = RendererUpdatePayload {
                timestamp,
                delta: instant_last.elapsed().as_micros() as u32,
                surface_size: *surface_size,
                stage_size: *stage_size,
                resource_manager: self.resource_manager.clone(),
            };

            let mut nodes: Vec<Arc<RwLock<dyn Node>>> = vec![];

            walk_nodes_top_bottom(&*root_node, &mut |child, parent| {
                let mut _child = child.write();
                _child.base_mut().update(parent.base(), &stage_size, false);

                let node_type = _child.node_type();

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
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            let childs: Vec<&dyn Node> = childs.iter().map(|n| &**n).collect();

            for child in childs {
                let node_type = child.node_type();

                if let Some(current_renderer) = renderers.get(node_type) {
                    render_pass.set_bind_group(0, &self.mvp_bind_group, &[]);
                    current_renderer.render(&device, &queue, &mut render_pass, child);
                }
            }
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
        window.pre_present_notify();

        queue.submit(
            std::iter::once(belt_encoder.finish()).chain(std::iter::once(encoder.finish())),
        );
        output.present();

        staging_belt.recall();

        Ok(())
    }

    #[inline(always)]
    pub fn input(&self, event: &WindowEvent) -> bool {
        let last_focused_node = self.last_focused_node.clone();

        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_focus_changes(position);

                self.last_focused_position
                    .store(Some(Arc::new(position.to_owned())));

                true
            }
            WindowEvent::CursorLeft { .. } => {
                let mut last_focused_node = last_focused_node.write();
                if let Some(last_focused_node) = &*last_focused_node {
                    dispatch_event(HaiEvent {
                        kind: HaiEventKind::MouseLeave,
                        target_id: *last_focused_node.target.read().base().id(),
                        bubble_target_ids: last_focused_node.parent_ids.clone(),
                    });
                }
                *last_focused_node = None;
                true
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if let Some(last_focused_position) = self.last_focused_position.load().as_ref() {
                    self.handle_focus_changes(last_focused_position);
                }

                if let Some(last_focused_node) = &*self.last_focused_node.read() {
                    let target_id = *last_focused_node.target.read().base().id();
                    let bubble_target_ids = last_focused_node.parent_ids.clone();

                    match state {
                        ElementState::Pressed => {
                            dispatch_event(HaiEvent {
                                kind: HaiEventKind::MouseDown,
                                target_id,
                                bubble_target_ids,
                            });
                            if let winit::event::MouseButton::Left = button {
                                self.mouse_down_id.store(target_id, Ordering::Relaxed);
                            }
                        }
                        ElementState::Released => {
                            dispatch_event(HaiEvent {
                                kind: HaiEventKind::MouseUp,
                                target_id,
                                bubble_target_ids: bubble_target_ids.clone(),
                            });

                            let pressed_target_id = self.mouse_down_id.swap(0, Ordering::Relaxed);
                            if pressed_target_id == target_id {
                                match button {
                                    winit::event::MouseButton::Left => {
                                        dispatch_event(HaiEvent {
                                            kind: HaiEventKind::Click,
                                            target_id,
                                            bubble_target_ids,
                                        });
                                    }
                                    winit::event::MouseButton::Right => {
                                        dispatch_event(HaiEvent {
                                            kind: HaiEventKind::ContextMenu,
                                            target_id,
                                            bubble_target_ids,
                                        });
                                    }
                                    winit::event::MouseButton::Back => {
                                        // do nothing
                                    }
                                    winit::event::MouseButton::Forward => {
                                        // do nothing
                                    }
                                    winit::event::MouseButton::Middle => {
                                        // do nothing
                                    }
                                    winit::event::MouseButton::Other(_) => {
                                        // do nothing
                                    }
                                }
                            }
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn handle_focus_changes(&self, position: &PhysicalPosition<f64>) {
        let root_node = &self.root_node;
        let last_focused_node = &self.last_focused_node;

        let stage_size = {
            let stage_size = self.stage_size.read();
            *stage_size
        };
        let surface_size = {
            let surface_size = self.surface_size.read();
            *surface_size
        };
        let scale_factor = stage_size.scale_factor();

        let global_logical_x = (position.x / scale_factor) as f32;
        let global_logical_y = (position.y / scale_factor) as f32;

        let upload_payload = FocusablePayload {
            surface_size,
            stage_size,
            resource_manager: self.resource_manager.clone(),
        };

        let mut last_focused_node = last_focused_node.write();

        // get node under pointer
        if let Some(node) = hit_test(
            root_node,
            global_logical_x,
            global_logical_y,
            &upload_payload,
        ) {
            if let Some(last_focused_node) = &*last_focused_node {
                // if last focused node is the same as current node, it's a mouse move event
                if last_focused_node == &node {
                    // TODO: mouse move event
                    // dispatch_event(HaiEvent {
                    //     kind: HaiEventKind::MouseMove,
                    //     target_id: *node.read().base().id(),
                    // });
                    return;
                }

                // if last focused node is different from current node, it's a mouse leave event and a mouse enter event
                dispatch_event(HaiEvent {
                    kind: HaiEventKind::MouseLeave,
                    target_id: *last_focused_node.target.read().base().id(),
                    bubble_target_ids: last_focused_node.parent_ids.clone(),
                });
                dispatch_event(HaiEvent {
                    kind: HaiEventKind::MouseEnter,
                    target_id: *node.target.read().base().id(),
                    bubble_target_ids: node.parent_ids.clone(),
                });
            } else {
                // if last focused node is None, it's only a mouse enter event
                dispatch_event(HaiEvent {
                    kind: HaiEventKind::MouseEnter,
                    target_id: *node.target.read().base().id(),
                    bubble_target_ids: node.parent_ids.clone(),
                });
            }

            // debug!("pointer is over {}", node.read().base().label());

            match node.target.read().base().cursor() {
                HaiCursor::Visible(cursor) => {
                    self.window.set_cursor_icon(*cursor);
                    self.window.set_cursor_visible(true);
                    debug!("set cursor to {}", cursor.name());
                }
                HaiCursor::Hidden => {
                    self.window.set_cursor_visible(false);
                    debug!("set cursor to hidden");
                }
            }

            // record last focused node
            *last_focused_node = Some(node);
        } else {
            // if no node under pointer, it's a mouse leave event if last focused node is not None
            if let Some(last_focused_node) = &*last_focused_node {
                // TODO: mouse leave event
                dispatch_event(HaiEvent {
                    kind: HaiEventKind::MouseLeave,
                    target_id: *last_focused_node.target.read().base().id(),
                    bubble_target_ids: last_focused_node.parent_ids.clone(),
                });

                self.window.set_cursor_icon(CursorIcon::Default);
                self.window.set_cursor_visible(true);
                debug!("set cursor to default");
            }

            *last_focused_node = None;
        }
    }
}
