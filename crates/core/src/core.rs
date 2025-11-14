mod global;
mod handle_events;
mod keyboard_events;
mod pointer_events;
mod render;

use arc_swap::{ArcSwap, ArcSwapOption};
use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use log::{debug, error};
use moyu_pal::config::{WindowState, get_engine_config};
use moyu_pal::sync::{Mutex, RwLock};
use moyu_pal::time::Instant;
use render::Graphics;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use winit::dpi::{LogicalSize, Size};
use winit::keyboard::ModifiersState;
use winit::window::{Fullscreen, Window};

use crate::base::*;
use crate::state::{MOUSE_IDENTIFIER, PointerState};
use crate::{nodes::Container, traits::*};

pub use self::global::*;

pub type NodeLock = Arc<RwLock<dyn Node>>;
pub type NodeMap = Arc<DashMap<u32, NodeLock>>;
pub type NodeRef<'a> = Ref<'a, u32, NodeLock>;
pub type PluginLock = Arc<Mutex<dyn Plugin>>;
pub type PluginRef<'a> = Ref<'a, String, PluginLock>;

pub struct Core {
    pub(crate) window: Arc<Window>,
    pub(crate) graphics: ArcSwapOption<Graphics>,
    #[cfg(native)]
    pub(crate) graphics_thread: ArcSwapOption<std::thread::JoinHandle<()>>,

    plugins: DashMap<String, PluginLock>,

    /// timer from program start
    pub instant: Instant,

    pub(crate) node_map: NodeMap,
    /// map for current pointer states, mouse event is stored in index -1 while touch events are stored in their identifier
    pub(crate) pointer_map: DashMap<i32, PointerState>,

    pub(crate) window_state: ArcSwap<WindowState>,
    pub(crate) cursor_state: ArcSwap<MoyuCursor>,
    pub(crate) modifiers_state: ArcSwap<ModifiersState>,

    /// Pause the rendering process, default is `false`
    pub(crate) is_paused: AtomicBool,
    /// Flag to indicate whether the application is about to quit.
    pub(crate) about_to_quit: AtomicBool,

    /// Size of current surface, which means the size of the window on desktop platforms, the size of the canvas \
    /// on web platform, and the size of the screen on mobile platforms.
    pub(crate) surface_size: Arc<RwLock<SurfaceSize>>,
    /// Size of stage, which is the content size set by user. Cannot be changed once set.
    pub(crate) stage_size: Arc<RwLock<SurfaceSize>>,
    pub(crate) stage_transform: Arc<RwLock<(f32, f32, f32)>>,
}

impl Core {
    pub fn new(window: Arc<Window>) -> Self {
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

        let surface_size = Arc::new(RwLock::new(surface_size));
        let stage_size = Arc::new(RwLock::new(stage_size));
        let stage_transform = Arc::new(RwLock::new((scale, translate_x, translate_y)));

        // create root node
        let root_node = Container::new("Root Node".to_string());
        let root_node = Arc::new(RwLock::new(root_node));

        let node_map: NodeMap = Default::default();
        node_map.insert(0, root_node);

        let pointer_map: DashMap<i32, PointerState> = Default::default();
        // add mouse pointer which is always there
        pointer_map.insert(MOUSE_IDENTIFIER, PointerState::default());

        Self {
            window,
            graphics: ArcSwapOption::empty(),
            #[cfg(native)]
            graphics_thread: ArcSwapOption::empty(),

            plugins: DashMap::new(),

            instant: Instant::now(),

            node_map,
            pointer_map,

            window_state: ArcSwap::new(Arc::new(WindowState::Idle)),
            cursor_state: ArcSwap::new(Arc::new(MoyuCursor::default())),
            modifiers_state: ArcSwap::new(Arc::new(ModifiersState::empty())),
            is_paused: AtomicBool::new(false),
            about_to_quit: AtomicBool::new(false),

            surface_size,
            stage_size,
            stage_transform,
        }
    }

    pub fn register_plugin(&self, name: &str, plugin: PluginLock) {
        if self.plugins.contains_key(name) {
            error!("There's already a plugin named '{}'.", name);
            return;
        }
        self.plugins.insert(name.to_owned(), plugin);
    }

    pub fn get_plugin(&self, name: &str) -> Option<PluginRef<'_>> {
        self.plugins.get(name)
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

    pub fn set_correct_canvas_size_for_web(&self) {
        // winit does not set the canvas size correctly on the web platform (it doesn't handle DPI),
        // so we follow the common web approach: create a canvas scaled by the DPI, and then shrink it
        // via CSS by 1/DPI.
        //
        // However, for now we do not support dynamic DPI changes (e.g. moving the browser window between monitors with
        // different DPIs).
        #[cfg(web)]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowExtWebSys;

            let window = &self.window;

            let dpi = window.scale_factor();

            let size = get_engine_config().initial_surface_size.as_tuple();
            let initial_surface_size = (size.0 as f64 * dpi, size.1 as f64 * dpi);

            let _ = window.request_inner_size(Size::Logical(initial_surface_size.into()));

            if let Some(canvas) = window.canvas() {
                canvas
                    .style()
                    .set_property("transform", &format!("scale({})", 1.0 / dpi))
                    .unwrap();
                canvas
                    .style()
                    .set_property("transform-origin", "top left")
                    .unwrap();

                // also set parent element size to remove empty areas
                if let Some(parent) = canvas.parent_element() {
                    if let Ok(parent) = parent.dyn_into::<web_sys::HtmlElement>() {
                        parent
                            .style()
                            .set_property("width", &format!("{}px", size.0))
                            .unwrap();
                        parent
                            .style()
                            .set_property("height", &format!("{}px", size.1))
                            .unwrap();
                        parent.style().set_property("overflow", "hidden").unwrap();
                    }
                }
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
            // see [Self::set_correct_canvas_size_for_web] for explanation about web platform
            let surface_size = if cfg!(web) {
                SurfaceSize::new(logical_width * factor, logical_height * factor, factor)
            } else {
                SurfaceSize::new(logical_width, logical_height, factor)
            };

            self.resize_stage(surface_size);

            let window_size = if cfg!(web) {
                Size::Logical(LogicalSize::new(
                    logical_width * factor,
                    logical_height * factor,
                ))
            } else {
                Size::Logical(LogicalSize::new(logical_width, logical_height))
            };

            let _ = window.request_inner_size(window_size);

            window.set_minimized(window_minimized.unwrap_or(false));
            window.set_maximized(window_maximized);

            // see [Self::set_correct_canvas_size_for_web] for explanation about web platform
            #[cfg(web)]
            {
                use wasm_bindgen::JsCast;
                use winit::platform::web::WindowExtWebSys;

                if let Some(canvas) = window.canvas() {
                    canvas
                        .style()
                        .set_property("transform", &format!("scale({})", 1.0 / factor))
                        .unwrap();
                    canvas
                        .style()
                        .set_property("transform-origin", "top left")
                        .unwrap();

                    if let Some(parent) = canvas.parent_element() {
                        if let Ok(parent) = parent.dyn_into::<web_sys::HtmlElement>() {
                            parent
                                .style()
                                .set_property("width", &format!("{}px", logical_width))
                                .unwrap();
                            parent
                                .style()
                                .set_property("height", &format!("{}px", logical_height))
                                .unwrap();
                            parent.style().set_property("overflow", "hidden").unwrap();
                        }
                    }
                }
            }

            // reset fullscreen status
            if window_fullscreen.is_some() {
                window.set_fullscreen(None);
                window.set_fullscreen(window_fullscreen);
            }
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

    #[cfg(web)]
    pub async fn init_graphics(&self) {
        let graphics = Graphics::init(
            &self.window,
            &self.surface_size(),
            &self.stage_size(),
            self.node_map.clone(),
        )
        .await;

        let graphics = Arc::new(graphics);
        self.graphics.store(Some(graphics.clone()));
    }

    #[cfg(native)]
    pub fn init_graphics(&self) {
        let graphics = moyu_pal::task::block_on_without_runtime(Graphics::init(
            &self.window,
            &self.surface_size.read(),
            &self.stage_size.read(),
            self.node_map.clone(),
        ));

        let graphics = Arc::new(graphics);
        self.graphics.store(Some(graphics.clone()));

        let graphics_thread = std::thread::Builder::new()
            .name("graphics".to_string())
            .spawn(move || {
                loop {
                    std::thread::park();
                    if let Err(err) = graphics.render() {
                        log::error!(
                            "Error occurs on rendering, terminate graphics thread: {:?}",
                            err
                        );
                        break;
                    }
                }
            })
            .expect("Failed to start graphics thread");

        self.graphics_thread.store(Some(Arc::new(graphics_thread)));
    }

    /// get whole node map
    pub fn node_map(&self) -> &NodeMap {
        &self.node_map
    }

    /// get root node
    pub fn root_node<'a>(&'a self) -> NodeRef<'a> {
        self.node_map.get(&0).unwrap()
    }

    pub fn graphics(&self) -> Option<Arc<Graphics>> {
        self.graphics.load().as_ref().cloned()
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

        let is_fullscreen = window.fullscreen().is_some();

        match state {
            WindowState::Idle => {
                window.set_maximized(false);
                window.set_minimized(false);
                if is_fullscreen {
                    window.set_fullscreen(None);
                }
            }
            WindowState::Maximized => {
                if is_fullscreen {
                    window.set_fullscreen(None);
                }
                window.set_maximized(true);
            }
            WindowState::Minimized => {
                if is_fullscreen {
                    window.set_fullscreen(None);
                }
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
    fn set_cursor(&self, cursor: MoyuCursor) {
        let prev_cursor = self.cursor_state.swap(Arc::new(cursor.clone()));

        if *prev_cursor != cursor {
            match cursor {
                MoyuCursor::Visible(cursor) => {
                    self.window.set_cursor(cursor);
                    self.window.set_cursor_visible(true);
                    debug!("set cursor to {}", cursor.name());
                }
                MoyuCursor::Hidden => {
                    self.window.set_cursor_visible(false);
                    debug!("set cursor to hidden");
                }
            }
        }
    }

    /// quit the application
    pub fn quit(&self) {
        self.about_to_quit.store(true, Ordering::Relaxed);
    }
}
