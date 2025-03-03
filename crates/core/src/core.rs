mod global;
mod handle_events;
mod keyboard_events;
mod pointer_events;
mod render;

use arc_swap::ArcSwap;
use doufu_pal::config::{get_engine_config, WindowState};
use doufu_pal::sync::{Mutex, RwLock};
use doufu_pal::time::Instant;
use log::{debug, error};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wgpu::util::{DeviceExt, StagingBelt};
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::dpi::{LogicalSize, Size};
use winit::event_loop::EventLoopProxy;
use winit::keyboard::ModifiersState;
use winit::window::{Fullscreen, Window};

use crate::base::*;
use crate::state::{PointerState, MOUSE_IDENTIFIER};
use crate::utils::fps_meter::FpsMeter;
use crate::{nodes::Container, resource::ResourceManager, traits::*, user_event::UserEvent};

pub use self::global::*;

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
    fps_meter: FpsMeter,
    /// timer from program start
    instant: Instant,
    /// time elapsed since last frame, in microseconds
    instant_last: ArcSwap<Instant>,

    pub(crate) root_node: Arc<RwLock<dyn Node>>,
    pub(crate) node_map: Arc<RwLock<HashMap<u32, Arc<RwLock<dyn Node>>>>>,
    /// map for current pointer states, mouse event is stored in index -1 while touch events are stored in their identifier
    pub(crate) pointer_map: Arc<RwLock<HashMap<i32, PointerState>>>,

    pub(crate) window_state: ArcSwap<WindowState>,
    pub(crate) cursor_state: ArcSwap<HaiCursor>,
    pub(crate) modifiers_state: ArcSwap<ModifiersState>,

    /// redraw mode, default is `Auto`
    pub(crate) redraw_mode: ArcSwap<HaiRedrawMode>,
    /// if `true`, the screen will be refreshed in next frame,
    /// by default it will be `true` to render every frame.
    pub(crate) is_dirty: AtomicBool,
    /// Pause the rendering process, default is `false`
    pub(crate) is_paused: AtomicBool,

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
    /// Size of stage, which is the content size set by user. Cannot be changed once set.
    pub(crate) stage_size: Arc<RwLock<SurfaceSize>>,
    pub(crate) stage_transform: Arc<RwLock<(f32, f32, f32)>>,
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
        let env = get_engine_config();

        // store surface and stage size
        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        let surface_size = SurfaceSize::from_physical_size(&size, scale_factor);

        let mut stage_size = SurfaceSize::default();
        stage_size.set_logical_size(
            env.stage_size.width() as f64,
            env.stage_size.height() as f64,
        );
        // use current monitor scale factor
        stage_size.set_scale_factor(scale_factor);

        let (scale, translate_x, translate_y) = get_scale_and_translate(
            env.stage_size.width() as f32,
            env.stage_size.height() as f32,
            size.width as f32,
            size.height as f32,
        );

        // create root node
        let root_node = Container::new("Root Node".to_string());
        let root_node = Arc::new(RwLock::new(root_node));

        let mut node_map: HashMap<u32, Arc<RwLock<dyn Node>>> = Default::default();
        node_map.insert(0, root_node.clone());

        let mut pointer_map: HashMap<i32, PointerState> = Default::default();
        // add mouse pointer which is always there
        pointer_map.insert(MOUSE_IDENTIFIER, PointerState::default());

        let resource_manager = ResourceManager::new(device.clone(), queue.clone());
        let renderers = HashMap::default();

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

        Self {
            instance,
            surface_size: Arc::new(RwLock::new(stage_size)),
            stage_size: Arc::new(RwLock::new(stage_size)),
            stage_transform: Arc::new(RwLock::new((scale, translate_x, translate_y))),
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
            fps_meter: FpsMeter::default(),
            instant: Instant::now(),
            instant_last: ArcSwap::new(Arc::new(Instant::now())),

            root_node,
            node_map: Arc::new(RwLock::new(node_map)),
            pointer_map: Arc::new(RwLock::new(pointer_map)),

            window_state: ArcSwap::new(Arc::new(WindowState::Idle)),
            cursor_state: ArcSwap::new(Arc::new(HaiCursor::default())),
            modifiers_state: ArcSwap::new(Arc::new(ModifiersState::empty())),
            redraw_mode: ArcSwap::new(Arc::new(HaiRedrawMode::Auto)),
            is_dirty: AtomicBool::new(true),
            is_paused: AtomicBool::new(false),

            after_render_handler: Arc::new(Mutex::new(None)),
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

    pub fn register_plugin(&self, name: &str, plugin: Arc<Mutex<dyn Plugin>>) {
        let mut plugins = self.plugins.lock();
        if plugins.contains_key(name) {
            error!("There's already a plugin named '{}'.", name);
            return;
        }
        plugins.insert(name.to_owned(), plugin);
    }

    pub fn get_plugin(&self, name: &str) -> Option<Arc<Mutex<dyn Plugin>>> {
        let plugins = self.plugins.lock();
        plugins.get(name).cloned()
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

    /// Get window of winit. This is useful when you need to do some low-level operations.
    /// However, it may break the encapsulation of the framework, so use it with caution.
    pub fn window(&self) -> &Arc<Window> {
        &self.window
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

    /// Move window to center of the screen. Only works on desktop platforms.
    pub fn move_to_center(&self) {
        #[cfg(desktop)]
        {
            let window = &self.window;
            if let Some(monitor) = window.current_monitor() {
                let monitor_size = monitor.size();
                let window_size = window.outer_size();

                window.set_outer_position(winit::dpi::PhysicalPosition {
                    x: monitor_size.width.saturating_sub(window_size.width) as f64 / 2.
                        + monitor.position().x as f64,
                    y: monitor_size.height.saturating_sub(window_size.height) as f64 / 2.
                        + monitor.position().y as f64,
                });
            }
        }
    }

    /// resize window, should be called in main thread
    pub fn resize_window(&self, logical_width: f64, logical_height: f64, factor: Option<f64>) {
        let window = &self.window;
        let factor = factor.unwrap_or(window.scale_factor());

        let window_fullscreen = window.fullscreen();
        let window_minimized = window.is_minimized();
        let window_maximized = window.is_maximized();

        if logical_width > 0. && logical_height > 0. {
            let surface_size = SurfaceSize::new(logical_width, logical_height, factor);
            self.resize_stage(surface_size);

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
    pub fn resize_stage(&self, new_size: SurfaceSize) {
        let mut config = self.config.lock();

        if cfg!(web) {
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

        *(self.surface_size.write()) = new_size;

        let stage_size = self.stage_size.read().logical_size_f32();

        self.queue.write_buffer(
            &self.mvp_buffer,
            0,
            bytemuck::bytes_of(&MVPMatrix::from_logical_size(
                stage_size,
                new_size.logical_size_f32(),
            )),
        );

        let (scale, translate_x, translate_y) = get_scale_and_translate(
            stage_size.0,
            stage_size.1,
            new_size.logical_size_f32().0,
            new_size.logical_size_f32().1,
        );

        *(self.stage_transform.write()) = (scale, translate_x, translate_y);

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
        *self.stage_size.read()
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

    pub fn get_window_state(&self) -> WindowState {
        **self.window_state.load()
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

    #[inline]
    fn set_cursor(&self, cursor: HaiCursor) {
        let prev_cursor = self.cursor_state.swap(Arc::new(cursor.clone()));

        if *prev_cursor != cursor {
            match cursor {
                HaiCursor::Visible(cursor) => {
                    self.window.set_cursor_icon(cursor);
                    self.window.set_cursor_visible(true);
                    debug!("set cursor to {}", cursor.name());
                }
                HaiCursor::Hidden => {
                    self.window.set_cursor_visible(false);
                    debug!("set cursor to hidden");
                }
            }
        }
    }
}
