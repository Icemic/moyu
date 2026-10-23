use std::sync::Arc;

use moyu_app::ScriptRuntime;
use moyu_core::core::{Core, WindowSystem, WindowSystemEvent, get_core, set_core, try_get_core};
use moyu_core::setup;
use moyu_pal::platform;
use moyu_pal::sync::Mutex;
use moyu_pal::visible_hand::VisibleHand;
#[cfg(desktop)]
use moyu_steam::SteamPlugin;
#[cfg(native)]
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::StartCause;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};

use crate::winit_window_system::{WinitWindowSystem, create_window};

#[allow(dead_code)]
pub async fn main_entry(
    event_loop: EventLoop<ApplicationInitEvent>,
    #[cfg(web)] element_id: &str,
    #[cfg(desktop)] steam_plugin: Option<SteamPlugin>,
) {
    // hold the global variable lifetime using VisibleHand
    let _async_runtime_handle = platform::setup();

    setup();

    let event_proxy = event_loop.create_proxy();

    #[cfg(native)]
    moyu_runtime::set_vm_wake_hook(Arc::new({
        let event_proxy = event_proxy.clone();
        move || {
            let _ = event_proxy.send_event(ApplicationInitEvent::VmWake);
        }
    }));

    let mut app = WinitApplication::new(
        event_proxy,
        #[cfg(web)]
        element_id,
        #[cfg(desktop)]
        steam_plugin,
    );

    event_loop.run_app(&mut app).ok();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApplicationInitEvent {
    /// The window system is ready: assemble the shared runtime and load the script.
    Init,
    /// The entry script has loaded; the window can be shown and the frame loop starts.
    InitFinished,
    VmWake,
    Quit,
}

struct WinitApplication {
    event_proxy: EventLoopProxy<ApplicationInitEvent>,
    /// The window and its window system, created on the first `resumed`.
    window_system: Option<Arc<WinitWindowSystem>>,
    #[cfg(web)]
    element_id: String,
    #[cfg(desktop)]
    steam_plugin: Option<SteamPlugin>,
    #[cfg(native)]
    frame_interval: Duration,
    #[cfg(native)]
    next_redraw_at: Option<Instant>,

    initialized: bool,

    _vm_handle: Arc<Mutex<Option<ScriptRuntime>>>,
    _core_handle: Arc<Mutex<Option<VisibleHand<Arc<Core>>>>>,
}

impl WinitApplication {
    fn new(
        event_proxy: EventLoopProxy<ApplicationInitEvent>,
        #[cfg(web)] element_id: &str,
        #[cfg(desktop)] steam_plugin: Option<SteamPlugin>,
    ) -> Self {
        WinitApplication {
            event_proxy,
            window_system: None,
            #[cfg(web)]
            element_id: element_id.to_string(),
            #[cfg(desktop)]
            steam_plugin,
            #[cfg(native)]
            frame_interval: Duration::from_secs_f64(1.0 / 60.0),
            #[cfg(native)]
            next_redraw_at: None,
            initialized: false,
            _vm_handle: Arc::new(Mutex::new(None)),
            _core_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub fn core(&self) -> Option<&Arc<Core>> {
        try_get_core()
    }

    /// The window system of the current session.
    ///
    /// Only called from the callbacks that run after `resumed` has created it.
    fn window_system(&self) -> &Arc<WinitWindowSystem> {
        self.window_system
            .as_ref()
            .expect("the window system is created on resumed")
    }

    #[cfg(native)]
    fn refresh_frame_interval(&mut self) {
        let refresh_rate = self
            .window_system()
            .window()
            .current_monitor()
            .and_then(|monitor| monitor.refresh_rate_millihertz())
            .unwrap_or(60_000);

        let next_frame_interval = Duration::from_secs_f64(1000.0 / refresh_rate as f64);

        if self.frame_interval != next_frame_interval {
            log::info!(
                "Refresh rate changed from {:.1} to {:.1}",
                1. / self.frame_interval.as_secs_f64(),
                1. / next_frame_interval.as_secs_f64()
            );
            self.frame_interval = next_frame_interval;
            self.next_redraw_at = None;

            let metrics = self.window_system().surface_metrics();

            if metrics.physical_width > 0 && metrics.physical_height > 0 {
                self.schedule_next_redraw(Instant::now());
            }
        }
    }

    #[cfg(native)]
    fn schedule_next_redraw(&mut self, now: Instant) {
        let next = match self.next_redraw_at {
            Some(scheduled_at) => {
                let mut candidate = scheduled_at;

                while candidate <= now {
                    candidate = candidate.checked_add(self.frame_interval).unwrap_or(now);
                }

                candidate
            }
            None => now.checked_add(self.frame_interval).unwrap_or(now),
        };

        self.next_redraw_at = Some(next);
    }

    /// Show the window and start the frame loop. Runs once the entry script has loaded,
    /// which is when the project's own content exists and a frame can be drawn.
    fn finish_start(&mut self) {
        let core = get_core();

        #[cfg(desktop)]
        core.move_to_center();

        // A native window already has its configured size here, while a web canvas
        // follows the box the page gives it, which is only known once layout has run.
        // Size changes after this point arrive as resize events.
        core.sync_surface_size_with_window(false);

        self.window_system().set_visible(true);
        self.window_system().request_redraw();

        #[cfg(native)]
        {
            let now = Instant::now();
            self.schedule_next_redraw(now);
        }

        moyu_app::announce_ready(core.clone());

        self.initialized = true;
    }
}

impl ApplicationHandler<ApplicationInitEvent> for WinitApplication {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        #[cfg(native)]
        if matches!(cause, StartCause::ResumeTimeReached { .. })
            && self.initialized
            && self.next_redraw_at.is_some()
        {
            let window_system = self.window_system();
            let metrics = window_system.surface_metrics();

            if metrics.physical_width > 0 && metrics.physical_height > 0 {
                window_system.request_redraw();
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ApplicationInitEvent) {
        match event {
            ApplicationInitEvent::VmWake =>
            {
                #[cfg(native)]
                if let Some(vm) = moyu_runtime::try_get_vm() {
                    vm.tick();
                }
            }
            ApplicationInitEvent::Init => {
                let core = get_core().clone();

                #[cfg(native)]
                {
                    // Poll on this thread so the plugin set and the script VM below share
                    // one thread: the VM is created here and ticked here by VmWake.
                    moyu_pal::task::block_on_without_runtime(moyu_app::prepare(&core));

                    // Steam stays with the host: its SDK has to be initialized before the
                    // window is created, and the plugin handle belongs to this runtime.
                    #[cfg(desktop)]
                    if let Some(plugin) = self.steam_plugin.take() {
                        core.register_plugin("steam", Arc::new(Mutex::new(plugin)));
                    }

                    let event_proxy = self.event_proxy.clone();
                    let vm_handle = match moyu_app::load_user_script(&core, move || {
                        log::info!("User script loaded.");
                        let _ = event_proxy.send_event(ApplicationInitEvent::InitFinished);
                    }) {
                        Ok(handle) => handle,
                        Err(err) => {
                            log::error!("{}", err);
                            platform::show_fatal_error_and_exit(err.to_string().as_str());
                        }
                    };
                    self._vm_handle.lock().replace(vm_handle);
                }

                #[cfg(web)]
                {
                    let event_proxy = self.event_proxy.clone();
                    moyu_pal::task::spawn(async move {
                        moyu_app::prepare(&core).await;

                        if let Err(err) = moyu_app::load_user_script(&core, move || {
                            log::info!("User script loaded.");
                            let _ = event_proxy.send_event(ApplicationInitEvent::InitFinished);
                        }) {
                            log::error!("{}", err);
                            platform::show_fatal_error_and_exit(err.to_string().as_str());
                        }
                    });
                }
            }
            ApplicationInitEvent::InitFinished => {
                self.finish_start();
            }
            ApplicationInitEvent::Quit => {
                event_loop.exit();
            }
        }
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.core().is_some() {
            #[cfg(native)]
            self.refresh_frame_interval();

            #[cfg(mobile)]
            if let Some(core) = self.core() {
                core.handle_window_system_event(WindowSystemEvent::Resumed);
            }

            self.window_system().set_visible(true);
            self.window_system().request_redraw();

            return;
        }

        let window = create_window(
            event_loop,
            #[cfg(web)]
            self.element_id.as_str(),
        );

        #[cfg(windows)]
        {
            use winit::platform::windows::IconExtWindows;
            use winit::window::Icon;
            if let Ok(icon) = Icon::from_resource(1, None) {
                window.set_window_icon(Some(icon));
            }
        }

        let window_system = Arc::new(WinitWindowSystem::new(window, self.event_proxy.clone()));
        self.window_system = Some(window_system.clone());

        #[cfg(native)]
        self.refresh_frame_interval();

        let core = Arc::new(Core::new(window_system));

        let _core_handle = set_core(core.clone());
        self._core_handle.lock().replace(_core_handle);

        // The debug bridge answers requests about the core, so it starts once the core exists.
        moyu_debugger::start();

        self.event_proxy
            .send_event(ApplicationInitEvent::Init)
            .unwrap();
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        #[cfg(native)]
        {
            self.next_redraw_at = None;
        }
        #[cfg(mobile)]
        if let Some(core) = self.core() {
            core.handle_window_system_event(WindowSystemEvent::Suspended);
        }
        log::warn!("Suspended");
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(core) = self.core() {
            core.handle_window_system_event(WindowSystemEvent::Idle);
        }

        #[cfg(native)]
        {
            let control_flow = if self.initialized {
                self.next_redraw_at
                    .map(ControlFlow::WaitUntil)
                    .unwrap_or(ControlFlow::Wait)
            } else {
                ControlFlow::Wait
            };
            event_loop.set_control_flow(control_flow);
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if !self.initialized {
            return;
        }

        if window_id != self.window_system().window().id() {
            return;
        }

        if let Some(system_event) = self.window_system().convert_window_event(&event)
            && let Some(core) = self.core()
        {
            core.handle_window_system_event(system_event);
        }

        #[cfg(native)]
        match &event {
            WindowEvent::RedrawRequested => {
                let metrics = self.window_system().surface_metrics();

                if metrics.physical_width == 0 || metrics.physical_height == 0 {
                    self.next_redraw_at = None;
                } else {
                    let now = Instant::now();
                    self.schedule_next_redraw(now);
                }
            }
            WindowEvent::Moved(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.refresh_frame_interval();
            }
            _ => {}
        }
    }
}
