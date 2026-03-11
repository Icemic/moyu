use std::sync::Arc;

use moyu_audio::AudioManager;
use moyu_core::core::{Core, get_core, set_core, try_get_core};
use moyu_core::events::GameEvent;
use moyu_core::nodes::Container;
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
use moyu_nodes::nodes::{Animation, Backdrop, Clip, Filter, Sprite, Text, Video};
use moyu_nodes::renderer::{
    AnimationRenderer, BackdropRenderer, ClipRenderer, OffscreenPassRenderer, SpriteRenderer,
    TextRenderer, VideoRenderer,
};
use moyu_pal::config::get_engine_config;
use moyu_pal::platform;
use moyu_pal::sync::Mutex;
use moyu_pal::visible_hand::VisibleHand;
#[cfg(native)]
use moyu_runtime::QuickVM;
use moyu_scenario::ScenarioPlugin;

#[allow(dead_code)]
pub async fn main_entry(event_loop: EventLoop<ApplicationInitEvent>, #[cfg(web)] element_id: &str) {
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
pub(crate) enum ApplicationInitEvent {
    #[default]
    Start,
    Graphic,
    Plugin,
    LoadUserScript,
    ShowAndStart,
}

struct Application {
    event_proxy: EventLoopProxy<ApplicationInitEvent>,
    #[cfg(web)]
    element_id: String,
    #[cfg(native)]
    loop_helper: Option<spin_sleep_util::Interval>,

    initialized: bool,

    #[cfg(native)]
    _vm_handle: Arc<Mutex<Option<VisibleHand<Arc<QuickVM>>>>>,
    #[cfg(web)]
    _vm_handle: Arc<Mutex<Option<()>>>,
    _core_handle: Arc<Mutex<Option<VisibleHand<Arc<Core>>>>>,
}

impl Application {
    fn new(
        event_proxy: EventLoopProxy<ApplicationInitEvent>,
        #[cfg(web)] element_id: &str,
    ) -> Self {
        Application {
            event_proxy,
            #[cfg(web)]
            element_id: element_id.to_string(),
            #[cfg(native)]
            loop_helper: None,
            initialized: false,
            _vm_handle: Arc::new(Mutex::new(None)),
            _core_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub fn core(&self) -> Option<&Arc<Core>> {
        try_get_core()
    }
}

impl ApplicationHandler<ApplicationInitEvent> for Application {
    fn user_event(&mut self, _: &ActiveEventLoop, event: ApplicationInitEvent) {
        match event {
            ApplicationInitEvent::Start => {
                // do nothing
            }
            ApplicationInitEvent::Graphic => {
                let core = get_core().clone();

                #[cfg(native)]
                {
                    core.init_graphics();
                    self.event_proxy
                        .send_event(ApplicationInitEvent::Plugin)
                        .unwrap();
                }

                #[cfg(web)]
                {
                    let event_proxy = self.event_proxy.clone();
                    moyu_pal::task::spawn(async move {
                        core.init_graphics().await;
                        event_proxy
                            .send_event(ApplicationInitEvent::Plugin)
                            .unwrap();
                    });
                }
            }
            ApplicationInitEvent::Plugin => {
                let core = get_core();

                core.register_node_type::<Container>("container");
                core.register_node_type::<Sprite>("sprite");
                core.register_node_type::<Text>("text");
                core.register_node_type::<Clip>("clip");
                core.register_node_type::<Filter>("filter");
                core.register_node_type::<Backdrop>("backdrop");
                core.register_node_type::<Animation>("animation");
                core.register_node_type::<Video>("video");

                if let Some(graphics) = core.graphics() {
                    let device = graphics.device();
                    let config = graphics.config().lock().clone();

                    let sprite_renderer = SpriteRenderer::new(&device, &config);
                    let text_renderer = TextRenderer::new(&device, &config);
                    let clip_renderer = ClipRenderer::new(&device, &config);
                    let filter_renderer = OffscreenPassRenderer::new(&device, &config);
                    let backdrop_renderer = BackdropRenderer::new(&device, &config);
                    let animation_renderer = AnimationRenderer::new(&device, &config);
                    let video_renderer = VideoRenderer::new(&device, &config);

                    text_renderer.init_huozi_from_env();

                    graphics.register_renderer("sprite", Box::new(sprite_renderer));
                    graphics.register_renderer("text", Box::new(text_renderer));
                    graphics.register_renderer("clip", Box::new(clip_renderer));
                    graphics.register_renderer("filter", Box::new(filter_renderer));
                    graphics.register_renderer("backdrop", Box::new(backdrop_renderer));
                    graphics.register_renderer("animation", Box::new(animation_renderer));
                    graphics.register_renderer("video", Box::new(video_renderer));
                }

                let core = get_core().clone();
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
                    event_proxy
                        .send_event(ApplicationInitEvent::LoadUserScript)
                        .unwrap();
                });
            }
            ApplicationInitEvent::LoadUserScript => {
                let core = get_core();
                let event_proxy = self.event_proxy.clone();
                // All plugins are ready, now we can spawn the runtime and execute scripts
                let _vm_handle = match moyu_ops::spawn::spawn_runtime_with_core(&core, move || {
                    log::info!("User script loaded.");
                    event_proxy
                        .send_event(ApplicationInitEvent::ShowAndStart)
                        .unwrap();
                }) {
                    Ok(v) => v,
                    Err(err) => {
                        log::error!("{}", err);
                        platform::show_fatal_error_and_exit(err.to_string().as_str());
                    }
                };
                self._vm_handle.lock().replace(_vm_handle);
            }
            ApplicationInitEvent::ShowAndStart => {
                let core = get_core();

                #[cfg(desktop)]
                core.move_to_center();

                core.window().set_visible(true);
                core.window().request_redraw();

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

                self.initialized = true;
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

        self.event_proxy
            .send_event(ApplicationInitEvent::Graphic)
            .unwrap();
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
        if !self.initialized {
            return;
        }

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
