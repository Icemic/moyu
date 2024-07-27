use std::sync::Arc;

use hai_audio::AudioManager;
use hai_core::core::set_core;
use hai_core::surface::{create_wgpu_surface, create_window};
use hai_core::user_event::UserEvent;
use hai_core::winit::dpi::PhysicalPosition;
use hai_core::winit::event::Event;
use hai_core::winit::event_loop::EventLoop;
use hai_core::winit::window::Window;
use hai_core::{create_hai_core, setup};
use hai_nodes::renderer::{SpriteRenderer, TextRenderer};
use hai_pal::platform;
use hai_pal::sync::Mutex;

#[allow(dead_code)]
pub fn main_entry(event_loop: EventLoop<UserEvent>) {
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

    #[cfg(not(feature = "web"))]
    let mut loop_helper = {
        // get max refresh rate of all monitors
        let mut refresh_rate_max = 60_000;
        for monitor in event_loop.available_monitors() {
            refresh_rate_max = refresh_rate_max.max(
                monitor
                    .refresh_rate_millihertz()
                    .map(|v| v)
                    .unwrap_or(refresh_rate_max),
            );
        }

        log::info!("max refresh rate: {}", refresh_rate_max / 1000);

        spin_sleep_util::interval(std::time::Duration::from_secs(1) / (refresh_rate_max / 1000))
    };

    event_loop
        .run(move |event, event_loop| {
            match event {
                Event::AboutToWait => {
                    #[cfg(not(feature = "web"))]
                    loop_helper.tick();
                }
                Event::Resumed => {
                    let _window = create_window(event_loop);

                    let (instance, surface, device, queue, config) = create_wgpu_surface(&_window);

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

                    // core.register_renderer("null".to_string(), null_renderer);
                    _core.register_renderer("sprite".to_string(), Box::new(sprite_renderer));
                    _core.register_renderer("text".to_string(), Box::new(text_renderer));

                    match AudioManager::new() {
                        Ok(audio_manager) => {
                            _core.register_plugin(
                                "audio".to_string(),
                                Arc::new(Mutex::new(audio_manager)),
                            );
                        }
                        Err(err) => {
                            log::error!("failed to create audio manager: {}", err);
                        }
                    }

                    // #[cfg(feature = "video")]
                    // core.register_renderer("video".to_string(), Box::new(video_renderer));

                    _core_handle = Some(set_core(_core.clone()));
                    _jsvm_handle = Some(hai_ops::spawn::spawn_runtime_with_core(&_core, None));

                    _window.set_visible(true);

                    // only for desktop platforms
                    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
                    if !_window.is_maximized() && !_window.fullscreen().is_some() {
                        move_to_center(&_window);
                    }

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

#[allow(dead_code)]
fn move_to_center(window: &Window) {
    if let Some(monitor) = window.current_monitor() {
        let monitor_size = monitor.size();
        let window_size = window.outer_size();

        window.set_outer_position(PhysicalPosition {
            x: monitor_size.width.saturating_sub(window_size.width) as f64 / 2.
                + monitor.position().x as f64,
            y: monitor_size.height.saturating_sub(window_size.height) as f64 / 2.
                + monitor.position().y as f64,
        });
    }
}
