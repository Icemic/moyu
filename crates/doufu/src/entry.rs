use std::sync::Arc;

use doufu_audio::AudioManager;
use doufu_core::core::set_core;
use doufu_core::plugins::SystemPlugin;
use doufu_core::surface::{create_wgpu_surface, create_window};
use doufu_core::user_event::UserEvent;
use doufu_core::winit::event::Event;
use doufu_core::winit::event_loop::EventLoop;
use doufu_core::{create_doufu_core, setup};
use doufu_nodes::renderer::{SpriteRenderer, TextRenderer};
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

    let mut window = None;
    let mut core = None;
    // hold the global variable lifetime using VisibleHand
    let mut _jsvm_handle = None;
    let mut _core_handle = None;

    #[cfg(native)]
    let mut loop_helper = {
        // get max refresh rate of all monitors
        let mut refresh_rate_max = 60_000;
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

    #[cfg(web)]
    {
        let _window = create_window(&event_loop, element_id);

        let (instance, surface, device, queue, config) = create_wgpu_surface(&_window).await;

        let sprite_renderer = SpriteRenderer::new(&device, &config);
        let text_renderer = TextRenderer::new(&device, &config);

        text_renderer.init_huozi_from_env();

        // use sprite renderer on video node
        // #[cfg(feature = "video")]
        // let video_renderer = SpriteRenderer::new(&device, &config);

        let _core = create_doufu_core(
            instance,
            surface,
            device,
            queue,
            config,
            &_window,
            event_proxy.clone(),
        );

        // core.register_renderer("null", null_renderer);
        _core.register_renderer("sprite", Box::new(sprite_renderer));
        _core.register_renderer("text", Box::new(text_renderer));

        match AudioManager::new() {
            Ok(audio_manager) => {
                _core.register_plugin("audio", Arc::new(Mutex::new(audio_manager)));
            }
            Err(err) => {
                log::error!("failed to create audio manager: {}", err);
            }
        }

        let scenario = ScenarioPlugin::new();
        let scenario = Arc::new(Mutex::new(scenario));
        let system = SystemPlugin::new(_core.clone());
        _core.register_plugin("scenario", scenario.clone());
        _core.register_plugin("system", Arc::new(Mutex::new(system)));

        if let Err(err) = scenario.lock().init().await {
            log::error!("Failed to initialize scenario plugin: {}", err);
        }

        // #[cfg(feature = "video")]
        // core.register_renderer("video", Box::new(video_renderer));

        _core_handle = Some(set_core(_core.clone()));
        _jsvm_handle = Some(doufu_ops::spawn::spawn_runtime_with_core(&_core, None));

        _window.set_visible(true);

        window = Some(_window);
        core = Some(_core);

        // use doufu_core::winit::platform::web::EventLoopExtWebSys;

        event_loop
            .run(move |event, event_loop| {
                match event {
                    Event::AboutToWait => {}
                    Event::Resumed => {
                        // workaround for Chrome since it doesn't apply the correct size
                        if let Some(ref window) = window {
                            let _ =
                                window.request_inner_size(doufu_core::winit::dpi::Size::Logical(
                                    doufu_pal::config::get_engine_config()
                                        .surface_size
                                        .as_tuple()
                                        .into(),
                                ));
                        }
                    }
                    Event::Suspended => {
                        unimplemented!("cannot handle Event::Suspended now.");
                    }
                    _ => {}
                }
                if let Some(ref window) = window {
                    if let Some(ref core) = core {
                        core.handle_events(&event, window, event_loop);
                    }
                }
            })
            .ok();
    }

    #[cfg(native)]
    event_loop
        .run(move |event, event_loop| {
            match event {
                Event::AboutToWait => {
                    loop_helper.tick();
                }
                Event::Resumed => {
                    let _window = create_window(event_loop);

                    let (instance, surface, device, queue, config) =
                        doufu_pal::task::block_on_without_runtime(create_wgpu_surface(&_window));

                    let sprite_renderer = SpriteRenderer::new(&device, &config);
                    let text_renderer = TextRenderer::new(&device, &config);

                    text_renderer.init_huozi_from_env();

                    // use sprite renderer on video node
                    // #[cfg(feature = "video")]
                    // let video_renderer = SpriteRenderer::new(&device, &config);

                    let _core = create_doufu_core(
                        instance,
                        surface,
                        device,
                        queue,
                        config,
                        &_window,
                        event_proxy.clone(),
                    );

                    // core.register_renderer("null", null_renderer);
                    _core.register_renderer("sprite", Box::new(sprite_renderer));
                    _core.register_renderer("text", Box::new(text_renderer));

                    match AudioManager::new() {
                        Ok(audio_manager) => {
                            _core.register_plugin("audio", Arc::new(Mutex::new(audio_manager)));
                        }
                        Err(err) => {
                            log::error!("failed to create audio manager: {}", err);
                        }
                    }

                    let scenario = ScenarioPlugin::new();
                    let scenario = Arc::new(Mutex::new(scenario));
                    let system = SystemPlugin::new(_core.clone());
                    _core.register_plugin("scenario", scenario.clone());
                    _core.register_plugin("system", Arc::new(Mutex::new(system)));

                    doufu_pal::task::block_on_without_runtime(async {
                        if let Err(err) = scenario.lock().init().await {
                            log::error!("failed to init scenario plugin: {}", err);
                        }
                    });

                    // #[cfg(feature = "video")]
                    // core.register_renderer("video", Box::new(video_renderer));

                    _core_handle = Some(set_core(_core.clone()));
                    _jsvm_handle = Some(doufu_ops::spawn::spawn_runtime_with_core(&_core, None));

                    _window.set_visible(true);

                    _core.move_to_center();

                    window = Some(_window);
                    core = Some(_core);
                }

                Event::Suspended => {
                    unimplemented!("cannot handle Event::Suspended now.");
                }
                _ => {}
            }
            if let Some(ref window) = window {
                if let Some(ref core) = core {
                    core.handle_events(&event, window, event_loop);
                }
            }
        })
        .ok();
}
