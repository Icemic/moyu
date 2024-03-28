use std::sync::Arc;

use hai_core::core::set_core;
use hai_core::surface::{create_eventloop, create_wgpu_surface, create_window};
use hai_core::winit::dpi::PhysicalPosition;
use hai_core::winit::event::Event;
use hai_core::winit::window::Window;
use hai_core::{create_hai_core, setup, spawn_runtime_with_core};
use hai_nodes::renderer::{SpriteRenderer, TextRenderer, YUVSpriteRenderer};
use hai_pal::{env, logger, platform};

fn main_entry() {
    env::setup();
    logger::setup();
    platform::setup();

    setup();

    let event_loop = create_eventloop();
    // create event proxy which allow us to send window events from another thread
    let event_proxy = event_loop.create_proxy();
    let event_proxy = Arc::new(event_proxy);

    let mut window = None;
    let mut core = None;

    let mut loop_helper = {
        // get max refresh rate of all monitors
        let mut refresh_rate_max: f64 = 60.0;
        for monitor in event_loop.available_monitors() {
            refresh_rate_max = refresh_rate_max.max(
                monitor
                    .refresh_rate_millihertz()
                    .map(|v| v as f64 / 1000.0)
                    .unwrap_or(60.0),
            );
        }

        log::info!("max refresh rate: {}", refresh_rate_max);

        // create loop helper with target refresh rate set to be double of max refresh rate
        spin_sleep::LoopHelper::builder().build_with_target_rate(refresh_rate_max * 2.0)
    };

    event_loop
        .run(move |event, event_loop| {
            loop_helper.loop_start();
            match event {
                Event::AboutToWait => {
                    loop_helper.loop_sleep();
                }
                Event::Resumed => {
                    let _window = create_window(event_loop);

                    let (instance, surface, device, queue, config) = create_wgpu_surface(&_window);

                    let _core = create_hai_core(
                        instance,
                        surface,
                        device,
                        queue,
                        config,
                        &_window,
                        event_proxy.clone(),
                    );

                    let sprite_renderer = SpriteRenderer::new(&device, &config);
                    let yuv_sprite_renderer = YUVSpriteRenderer::new(&device, &config);
                    let text_renderer = TextRenderer::new(&device, &config);

                    // use sprite renderer on video node
                    // #[cfg(feature = "video")]
                    // let video_renderer = SpriteRenderer::new(&device, &config);

                    // core.register_renderer("null".to_string(), null_renderer);
                    core.register_renderer("sprite".to_string(), Box::new(sprite_renderer));
                    core.register_renderer("yuv_sprite".to_string(), Box::new(yuv_sprite_renderer));
                    core.register_renderer("text".to_string(), Box::new(text_renderer));

                    // #[cfg(feature = "video")]
                    // core.register_renderer("video".to_string(), Box::new(video_renderer));

                    set_core(_core.clone());
                    spawn_runtime_with_core(&_core, None);

                    _window.set_visible(true);

                    move_to_center(&_window);

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

#[cfg(not(feature = "web"))]
#[tokio::main]
async fn main() {
    main_entry();
}

#[cfg(feature = "web")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(feature = "web")]
#[cfg_attr(feature = "web", wasm_bindgen)]
pub fn wasm_start() {
    main_entry();
}

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
