mod global;
mod handle_events;
mod keyboard_events;
mod pointer_events;
mod render;

use arc_swap::{ArcSwap, ArcSwapOption};
use doufu_pal::config::{get_engine_config, WindowState};
use doufu_pal::sync::{Mutex, RwLock};
use doufu_pal::time::Instant;
use log::{debug, error};
use render::Graphics;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use winit::dpi::{LogicalSize, Size};
use winit::event_loop::{EventLoop, EventLoopProxy};
use winit::keyboard::ModifiersState;
use winit::window::{Fullscreen, Window};

use crate::base::*;
use crate::state::{PointerState, MOUSE_IDENTIFIER};
use crate::surface::create_window;
use crate::{nodes::Container, traits::*, user_event::UserEvent};

pub use self::global::*;

pub struct Core {
    pub(crate) window: Arc<Window>,
    pub(crate) graphics: ArcSwapOption<Graphics>,
    #[cfg(native)]
    pub(crate) graphics_thread: ArcSwapOption<std::thread::JoinHandle<()>>,

    pub(crate) event_proxy: Arc<EventLoopProxy<UserEvent>>,

    plugins: Arc<Mutex<HashMap<String, Arc<Mutex<dyn Plugin>>>>>,

    /// timer from program start
    pub instant: Instant,

    pub(crate) root_node: Arc<RwLock<dyn Node>>,
    pub(crate) node_map: Arc<RwLock<HashMap<u32, Arc<RwLock<dyn Node>>>>>,
    /// map for current pointer states, mouse event is stored in index -1 while touch events are stored in their identifier
    pub(crate) pointer_map: Arc<RwLock<HashMap<i32, PointerState>>>,

    pub(crate) window_state: ArcSwap<WindowState>,
    pub(crate) cursor_state: ArcSwap<HaiCursor>,
    pub(crate) modifiers_state: ArcSwap<ModifiersState>,

    /// Pause the rendering process, default is `false`
    pub(crate) is_paused: AtomicBool,

    /// Size of current surface, which means the size of the window on desktop platforms, the size of the canvas \
    /// on web platform, and the size of the screen on mobile platforms.
    pub(crate) surface_size: Arc<RwLock<SurfaceSize>>,
    /// Size of stage, which is the content size set by user. Cannot be changed once set.
    pub(crate) stage_size: Arc<RwLock<SurfaceSize>>,
    pub(crate) stage_transform: Arc<RwLock<(f32, f32, f32)>>,
}

unsafe impl Send for Core {}
unsafe impl Sync for Core {}

impl Core {
    pub fn new<T>(
        event_loop: &EventLoop<T>,
        #[cfg(web)] element_id: &str,
        event_proxy: Arc<EventLoopProxy<UserEvent>>,
    ) -> Self {
        let env = get_engine_config();

        let window = create_window(
            event_loop,
            #[cfg(web)]
            element_id,
        );

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

        let surface_size = Arc::new(RwLock::new(surface_size));
        let stage_size = Arc::new(RwLock::new(stage_size));
        let stage_transform = Arc::new(RwLock::new((scale, translate_x, translate_y)));

        // create root node
        let root_node = Container::new("Root Node".to_string());
        let root_node = Arc::new(RwLock::new(root_node));

        let mut node_map: HashMap<u32, Arc<RwLock<dyn Node>>> = Default::default();
        node_map.insert(0, root_node.clone());

        let mut pointer_map: HashMap<i32, PointerState> = Default::default();
        // add mouse pointer which is always there
        pointer_map.insert(MOUSE_IDENTIFIER, PointerState::default());

        Self {
            window,
            graphics: ArcSwapOption::empty(),
            #[cfg(native)]
            graphics_thread: ArcSwapOption::empty(),

            event_proxy,

            plugins: Arc::new(Mutex::new(HashMap::new())),

            instant: Instant::now(),

            root_node,
            node_map: Arc::new(RwLock::new(node_map)),
            pointer_map: Arc::new(RwLock::new(pointer_map)),

            window_state: ArcSwap::new(Arc::new(WindowState::Idle)),
            cursor_state: ArcSwap::new(Arc::new(HaiCursor::default())),
            modifiers_state: ArcSwap::new(Arc::new(ModifiersState::empty())),
            is_paused: AtomicBool::new(false),

            surface_size,
            stage_size,
            stage_transform,
        }
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

    /// Get window of winit. This is useful when you need to do some low-level operations.
    /// However, it may break the encapsulation of the framework, so use it with caution.
    pub fn window(&self) -> &Arc<Window> {
        &self.window
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
        *(self.surface_size.write()) = new_size;

        let stage_size = self.stage_size.read().logical_size_f32();

        let (scale, translate_x, translate_y) = get_scale_and_translate(
            stage_size.0,
            stage_size.1,
            new_size.logical_size_f32().0,
            new_size.logical_size_f32().1,
        );

        *(self.stage_transform.write()) = (scale, translate_x, translate_y);

        if let Some(graphics) = self.graphics.load().as_ref() {
            graphics.reconfigure_surface(new_size, self.stage_size());
            log::info!("Surface reconfigured with new size: {:?}", new_size);
        } else {
            log::warn!("No graphics instance found, skipping surface reconfiguration.");
        }
    }

    pub fn init_graphics(&self) {
        let graphics = Graphics::init(
            &self.window,
            &self.surface_size.read(),
            &self.stage_size.read(),
            self.root_node.clone(),
        );

        let graphics = Arc::new(graphics);
        self.graphics.store(Some(graphics.clone()));

        #[cfg(native)]
        {
            let graphics_thread = std::thread::Builder::new()
                .name("graphics".to_string())
                .spawn(move || loop {
                    std::thread::park();
                    if let Err(err) = graphics.render() {
                        log::error!(
                            "Error occurs on rendering, terminate graphics thread: {:?}",
                            err
                        );
                        break;
                    }
                })
                .expect("Failed to start graphics thread");

            self.graphics_thread.store(Some(Arc::new(graphics_thread)));
        }
    }

    /// get whole node map
    pub fn node_map(&self) -> &Arc<RwLock<HashMap<u32, Arc<RwLock<dyn Node>>>>> {
        &self.node_map
    }

    /// get root node
    pub fn root_node(&self) -> &Arc<RwLock<dyn Node>> {
        &self.root_node
    }

    pub fn graphics(&self) -> Option<Arc<Graphics>> {
        self.graphics.load().as_ref().cloned()
    }

    /// dispatch user event
    pub fn send_event(&self, event: UserEvent) {
        if let Err(err) = self.event_proxy.send_event(event) {
            error!("Failed to send event: {:?}", err);
        }
    }

    pub fn surface_size(&self) -> SurfaceSize {
        *self.surface_size.read()
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
