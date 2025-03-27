use std::sync::Arc;

use doufu_audio::AudioManager;
use doufu_core::core::set_core;
use doufu_core::plugins::SystemPlugin;
use doufu_core::user_event::UserEvent;
use doufu_core::winit::event::{Event, WindowEvent};
use doufu_core::winit::event_loop::EventLoop;
use doufu_core::{create_doufu_core, setup};
use doufu_gamepad::GamepadPlugin;
use doufu_nodes::renderer::{SpriteRenderer, TextRenderer};
use doufu_pal::config::get_engine_config;
use doufu_pal::platform;
use doufu_pal::sync::Mutex;
use doufu_scenario::ScenarioPlugin;

#[allow(dead_code)]
pub async fn main_entry(event_loop: EventLoop<UserEvent>, #[cfg(web)] element_id: &str) {
    // hold the global variable lifetime using VisibleHand
    let _async_runtime_handle = platform::setup();

    setup();

    // create event proxy which allow us to send window events from another thread
    let event_proxy = event_loop.create_proxy();
    let event_proxy = Arc::new(event_proxy);

    let mut loop_helper = {
        // get max refresh rate of all monitors
        #[allow(unused_mut)]
        let mut refresh_rate_max = 60_000;

        // For web, there's no implementation for available_monitors
        #[cfg(native)]
        for monitor in event_loop.available_monitors() {
            refresh_rate_max = refresh_rate_max.max(
                monitor
                    .refresh_rate_millihertz()
                    .unwrap_or(refresh_rate_max),
            );
        }

        log::info!("max refresh rate: {}", refresh_rate_max / 1000);

        spin_sleep_util::interval(std::time::Duration::from_secs(1) / (refresh_rate_max / 1000))
    };

    let core = create_doufu_core(
        &event_loop,
        #[cfg(web)]
        element_id,
        event_proxy.clone(),
    );

    match AudioManager::new() {
        Ok(audio_manager) => {
            core.register_plugin("audio", Arc::new(Mutex::new(audio_manager)));
        }
        Err(err) => {
            log::error!("failed to create audio manager: {}", err);
        }
    }

    let scenario = ScenarioPlugin::new();
    let scenario = Arc::new(Mutex::new(scenario));
    let system = SystemPlugin::new(core.clone());
    core.register_plugin("scenario", scenario.clone());
    core.register_plugin("system", Arc::new(Mutex::new(system)));

    if let Err(err) = scenario.lock().init().await {
        log::error!("Failed to initialize scenario plugin: {}", err);
    }

    #[cfg(any(desktop, web))]
    if get_engine_config().enable_gamepads {
        log::info!("enable gamepad plugin");
        let gamepad = GamepadPlugin::new();
        core.register_plugin("gamepad", Arc::new(Mutex::new(gamepad)));
    }

    let _core_handle = set_core(core.clone());

    let _vm_handle = doufu_ops::spawn::spawn_runtime_with_core(&core, None);

    event_loop
        .run(move |event, event_loop| {
            match event {
                Event::Resumed => {
                    core.init_graphics();

                    if let Some(graphics) = core.graphics() {
                        let device = graphics.device();
                        let config = graphics.config().lock().clone();

                        let sprite_renderer = SpriteRenderer::new(&device, &config);
                        let text_renderer = TextRenderer::new(&device, &config);

                        text_renderer.init_huozi_from_env();

                        graphics.register_renderer("sprite", Box::new(sprite_renderer));
                        graphics.register_renderer("text", Box::new(text_renderer));
                    }

                    // workaround for Chrome since it doesn't apply the correct size
                    #[cfg(web)]
                    let _ =
                        core.window()
                            .request_inner_size(doufu_core::winit::dpi::Size::Logical(
                                doufu_pal::config::get_engine_config()
                                    .surface_size
                                    .as_tuple()
                                    .into(),
                            ));

                    core.window().set_visible(true);

                    #[cfg(desktop)]
                    core.move_to_center();
                }
                Event::Suspended => {
                    log::warn!("Suspended");
                }
                _ => {}
            }

            core.handle_events(&event, core.window(), event_loop);

            if let Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } = &event
            {
                loop_helper.tick();
            }
        })
        .ok();
}
