use std::sync::Arc;

use arc_swap::ArcSwap;
use moyu_audio::AudioManager;
use moyu_core::core::{Core, get_core, set_core, try_get_core};
use moyu_core::events::GameEvent;
use moyu_core::plugins::SystemPlugin;
use moyu_core::setup;
use moyu_core::surface::create_window;
use moyu_core::utils::dispatch_event::dispatch_event;
use moyu_core::winit::application::ApplicationHandler;
#[cfg(native)]
use moyu_core::winit::event::WindowEvent;
use moyu_core::winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
#[cfg(any(desktop, web))]
use moyu_gamepad::GamepadPlugin;
use moyu_nodes::renderer::{SpriteRenderer, TextRenderer};
use moyu_pal::config::get_engine_config;
use moyu_pal::platform;
use moyu_pal::sync::Mutex;
use moyu_pal::visible_hand::VisibleHand;
#[cfg(native)]
use moyu_runtime::QuickVM;
use moyu_scenario::ScenarioPlugin;

#[allow(dead_code)]
pub async fn main_entry(event_loop: EventLoop<()>, #[cfg(web)] element_id: &str) {
    // hold the global variable lifetime using VisibleHand
    let _async_runtime_handle = platform::setup();

    setup();

    let event_proxy = event_loop.create_proxy();

    let mut app = Application::new(
        event_proxy,
        #[cfg(web)]
        element_id,
    );

    event_loop.run_app(&mut app).ok();
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum ApplicationInitState {
    #[default]
    Start,
    Graphic,
    Plugin,
    ShowAndRun,
}

struct Application {
    event_proxy: EventLoopProxy<()>,
    #[cfg(web)]
    element_id: String,
    #[cfg(native)]
    loop_helper: Option<spin_sleep_util::Interval>,

    init_state: Arc<ArcSwap<ApplicationInitState>>,

    #[cfg(native)]
    _vm_handle: Arc<Mutex<Option<VisibleHand<Arc<QuickVM>>>>>,
    #[cfg(web)]
    _vm_handle: Arc<Mutex<Option<()>>>,
    _core_handle: Arc<Mutex<Option<VisibleHand<Arc<Core>>>>>,
}

impl Application {
    fn new(event_proxy: EventLoopProxy<()>, #[cfg(web)] element_id: &str) -> Self {
        Application {
            event_proxy,
            #[cfg(web)]
            element_id: element_id.to_string(),
            #[cfg(native)]
            loop_helper: None,
            init_state: Arc::new(ArcSwap::from_pointee(ApplicationInitState::Start)),
            _vm_handle: Arc::new(Mutex::new(None)),
            _core_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub fn core(&self) -> Option<&Arc<Core>> {
        try_get_core()
    }
}

impl ApplicationHandler for Application {
    fn user_event(&mut self, _: &ActiveEventLoop, _: ()) {
        match self.init_state.load().as_ref() {
            ApplicationInitState::Start => {
                // do nothing
            }
            ApplicationInitState::Graphic => {
                let core = get_core().clone();

                #[cfg(native)]
                {
                    core.init_graphics();
                    self.init_state
                        .store(Arc::new(ApplicationInitState::Plugin));
                    self.event_proxy.send_event(()).unwrap();
                }

                #[cfg(web)]
                {
                    let state = self.init_state.clone();
                    let event_proxy = self.event_proxy.clone();
                    moyu_pal::task::spawn(async move {
                        core.init_graphics().await;
                        state.store(Arc::new(ApplicationInitState::Plugin));
                        event_proxy.send_event(()).unwrap();
                    });
                }
            }
            ApplicationInitState::Plugin => {
                let core = get_core();
                if let Some(graphics) = core.graphics() {
                    let device = graphics.device();
                    let config = graphics.config().lock().clone();

                    let sprite_renderer = SpriteRenderer::new(&device, &config);
                    let text_renderer = TextRenderer::new(&device, &config);

                    text_renderer.init_huozi_from_env();

                    graphics.register_renderer("sprite", Box::new(sprite_renderer));
                    graphics.register_renderer("text", Box::new(text_renderer));
                }

                let core = get_core().clone();
                let state = self.init_state.clone();
                let event_proxy = self.event_proxy.clone();
                moyu_pal::task::spawn(async move {
                    match AudioManager::new() {
                        Ok(audio_manager) => {
                            core.register_plugin("audio", Arc::new(Mutex::new(audio_manager)));
                        }
                        Err(err) => {
                            log::error!("failed to create audio manager: {}", err);
                        }
                    }

                    let system = SystemPlugin::new(core.clone());
                    core.register_plugin("system", Arc::new(Mutex::new(system)));

                    let mut scenario = ScenarioPlugin::new();
                    if let Err(err) = scenario.init().await {
                        log::error!("Failed to initialize scenario plugin: {}", err);
                    }
                    let scenario = Arc::new(Mutex::new(scenario));
                    core.register_plugin("scenario", scenario.clone());

                    #[cfg(any(desktop, web))]
                    if get_engine_config().enable_gamepads {
                        log::info!("enable gamepad plugin");
                        let gamepad = GamepadPlugin::new();
                        core.register_plugin("gamepad", Arc::new(Mutex::new(gamepad)));
                    }
                    state.store(Arc::new(ApplicationInitState::ShowAndRun));
                    event_proxy.send_event(()).unwrap();
                });
            }
            ApplicationInitState::ShowAndRun => {
                let core = get_core();
                // All plugins are ready, now we can spawn the runtime and execute scripts
                let _vm_handle = match moyu_ops::spawn::spawn_runtime_with_core(&core) {
                    Ok(v) => v,
                    Err(err) => {
                        log::error!("{}", err);
                        platform::show_fatal_error_and_exit(err.to_string().as_str());
                    }
                };
                self._vm_handle.lock().replace(_vm_handle);

                // workaround for Chrome since it doesn't apply the correct size
                #[cfg(web)]
                core.set_correct_canvas_size_for_web();

                #[cfg(desktop)]
                core.move_to_center();

                core.window().set_visible(true);

                // show splash screen
                if !get_engine_config().skip_splash {
                    let core = core.clone();
                    moyu_pal::task::spawn(async move {
                        crate::splash::show_splash_screen(core).await;
                        // tell script that engine is ready to render
                        dispatch_event(GameEvent::Ready);
                    });
                } else {
                    // tell script that engine is ready to render
                    dispatch_event(GameEvent::Ready);
                }
            }
        }
    }
    fn resumed(&mut self, event_loop: &moyu_core::winit::event_loop::ActiveEventLoop) {
        #[cfg(native)]
        {
            // get max refresh rate of all monitors
            #[allow(unused_mut)]
            let mut refresh_rate_max = 60_000;

            // For web, there's no implementation for available_monitors
            for monitor in event_loop.available_monitors() {
                refresh_rate_max = refresh_rate_max.max(
                    monitor
                        .refresh_rate_millihertz()
                        .unwrap_or(refresh_rate_max),
                );
            }

            log::info!("max refresh rate: {}", refresh_rate_max / 1000);

            let loop_helper = spin_sleep_util::interval(
                std::time::Duration::from_secs(1) / (refresh_rate_max / 1000),
            );
            self.loop_helper = Some(loop_helper);
        };

        let window = create_window(
            event_loop,
            #[cfg(web)]
            self.element_id.as_str(),
        );

        let core = Core::new(window);
        let core = Arc::new(core);

        let _core_handle = set_core(core.clone());
        self._core_handle.lock().replace(_core_handle);

        self.init_state
            .store(Arc::new(ApplicationInitState::Graphic));
        self.event_proxy.send_event(()).unwrap();
    }

    fn suspended(&mut self, _: &moyu_core::winit::event_loop::ActiveEventLoop) {
        log::warn!("Suspended");
    }

    fn about_to_wait(&mut self, event_loop: &moyu_core::winit::event_loop::ActiveEventLoop) {
        if let Some(core) = self.core() {
            core.handle_about_to_wait(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &moyu_core::winit::event_loop::ActiveEventLoop,
        window_id: moyu_core::winit::window::WindowId,
        event: moyu_core::winit::event::WindowEvent,
    ) {
        if let Some(core) = self.core() {
            core.handle_window_event(&event, &window_id, event_loop);
        }

        #[cfg(native)]
        if let WindowEvent::RedrawRequested = event {
            if let Some(loop_helper) = &mut self.loop_helper {
                loop_helper.tick();
            }
        }
    }
}
