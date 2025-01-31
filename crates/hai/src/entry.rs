use std::sync::Arc;

use hai_audio::AudioManager;
use hai_core::core::set_core;
use hai_core::surface::{create_wgpu_surface, create_window};
use hai_core::user_event::UserEvent;
use hai_core::winit::event::Event;
use hai_core::winit::event_loop::EventLoop;
use hai_core::{create_hai_core, setup};
use hai_nodes::renderer::{SpriteRenderer, TextRenderer};
use hai_pal::platform;
use hai_pal::sync::Mutex;
use hai_scenario::ScenarioPlugin;

#[allow(dead_code)]
pub async fn main_entry(event_loop: EventLoop<UserEvent>) {
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
        let _window = create_window(&event_loop);

        let (instance, surface, device, queue, config) = create_wgpu_surface(&_window).await;

        let sprite_renderer = SpriteRenderer::new(&device, &config);
        let text_renderer = TextRenderer::new(&device, &config);

        text_renderer.init_huozi_from_env();

        // use sprite renderer on video node
        // #[cfg(feature = "video")]
        // let video_renderer = SpriteRenderer::new(&device, &config);

        let _core = create_hai_core(
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

        // #[cfg(feature = "video")]
        // core.register_renderer("video", Box::new(video_renderer));

        _core_handle = Some(set_core(_core.clone()));
        _jsvm_handle = Some(hai_ops::spawn::spawn_runtime_with_core(&_core, None));

        _window.set_visible(true);

        window = Some(_window);
        core = Some(_core);

        // use hai_core::winit::platform::web::EventLoopExtWebSys;

        event_loop
            .run(move |event, event_loop| {
                match event {
                    Event::AboutToWait => {}
                    Event::Resumed => {
                        // workaround for Chrome since it doesn't apply the correct size
                        if let Some(ref window) = window {
                            let _ = window.request_inner_size(hai_core::winit::dpi::Size::Logical(
                                hai_pal::config::get_engine_config()
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
                        hai_pal::task::block_on_without_runtime(create_wgpu_surface(&_window));

                    let sprite_renderer = SpriteRenderer::new(&device, &config);
                    let text_renderer = TextRenderer::new(&device, &config);

                    text_renderer.init_huozi_from_env();

                    // use sprite renderer on video node
                    // #[cfg(feature = "video")]
                    // let video_renderer = SpriteRenderer::new(&device, &config);

                    let _core = create_hai_core(
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
                    _core.register_plugin("scenario", Arc::new(Mutex::new(scenario)));

                    // #[cfg(feature = "video")]
                    // core.register_renderer("video", Box::new(video_renderer));

                    _core_handle = Some(set_core(_core.clone()));
                    _jsvm_handle = Some(hai_ops::spawn::spawn_runtime_with_core(&_core, None));

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
