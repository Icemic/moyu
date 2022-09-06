#![feature(drain_filter)]

mod nodes;
mod ops;
mod presets;
mod renderer;
mod resource;
mod state;
mod traits;
mod types;
mod user_event;

use cgmath::num_traits::ToPrimitive;
#[cfg(not(target_arch = "wasm32"))]
use hai_js_runtime::JSRuntime;
use hai_pal::{env, logger, platform};
use log::{error, info};
use renderer::{create_surface, input, Renderer, SpriteRenderer};
use state::State;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
use std::{
    process::exit,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use user_event::UserEvent;
use winit::{
    dpi::{LogicalSize, Size},
    event::*,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::WindowBuilder,
};

fn main() {
    platform::setup();
    env::setup();
    logger::setup();

    // web target only
    // add a canvas element to dom as 'window'
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
    }

    // create main thread infinity loop
    let event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build();
    // create window
    let window = WindowBuilder::new()
        .with_inner_size(Size::Logical(LogicalSize::new(1280., 720.)))
        .with_resizable(false)
        .with_visible(false)
        .build(&event_loop)
        .unwrap();

    // create wgpu surface
    #[cfg(not(target_arch = "wasm32"))]
    let (surface, device, queue, config) =
        futures::executor::block_on(create_surface(&window, &window.inner_size()));
    #[cfg(target_arch = "wasm32")]
    let (surface, device, queue, config) = { pollster::block_on(create_surface(&window, &size)) };

    let sprite_renderer = SpriteRenderer::new(&device, &config);

    let surface = Arc::new(Mutex::new(surface));
    let device = Arc::new(Mutex::new(device));
    let queue = Arc::new(Mutex::new(queue));

    let event_proxy = event_loop.create_proxy();

    // create multithread shared state
    let mut state = State::new(surface, device, queue, config, event_proxy);

    // state.register_renderer("null".to_string(), null_renderer);
    state.register_renderer("sprite".to_string(), Box::new(sprite_renderer));

    // set screen size
    let size = window.inner_size();
    let scale_factor = window.scale_factor();
    state.set_screen_size((size.width, size.height), scale_factor);

    // make state sharable among threads
    let state = Arc::new(Mutex::new(state));

    // desktop targets only
    // spawn a v8 thread
    #[cfg(not(target_arch = "wasm32"))]
    {
        let state = state.clone();

        thread::spawn(|| {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            let resource_manager = {
                let state = state.lock().unwrap();
                state.resource_manager.clone()
            };
            runtime.block_on(async {
                let mut vm = JSRuntime::new(state);

                vm.with_global(|scope, global| {
                    ops::init(scope, global);
                });

                if let Err(err) = vm.prepare_entry().await {
                    error!("{}", err.to_string());
                    exit(-1);
                };

                vm.run_event_loop(|cx| {
                    let mut resource_manager = resource_manager.lock().unwrap();
                    resource_manager.poll(cx)
                })
                .await;
            });
        });
    }

    window.set_visible(true);

    let mut fps_requested = 0;
    let mut fps_rendered = 0;
    let mut last_fps_timestamp = 0.;

    let mut renderer = Renderer::new();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                match renderer.render(&state) {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => {
                        let mut state = state.lock().unwrap();
                        state.refresh();
                    }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }

                fps_rendered += 1;
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();

                fps_requested += 1;

                let time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64();
                let delta = time - last_fps_timestamp;
                if delta >= 1. {
                    window.set_title(
                        format!(
                            "fps: {:.1} requested, {:.1} rendered",
                            fps_requested as f64 / delta,
                            fps_rendered as f64 / delta
                        )
                        .as_str(),
                    );
                    last_fps_timestamp = time;
                    fps_rendered = 0;
                    fps_requested = 0;
                }
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                // makes State to have priority over main()
                if !input(event, &state) {
                    // UPDATED!
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            let mut state = state.lock().unwrap();
                            state.resize((physical_size.width, physical_size.height), None);
                        }
                        WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            new_inner_size,
                            ..
                        } => {
                            let mut state = state.lock().unwrap();
                            state.resize(
                                (new_inner_size.width, new_inner_size.height),
                                Some(*scale_factor),
                            );
                        }
                        _ => {}
                    }
                }
            }
            Event::UserEvent(user_event) => match user_event {
                UserEvent::ResizeWindow(logical_width, logical_height, factor) => {
                    let factor = factor.unwrap_or(window.scale_factor());
                    let mut state = state.lock().unwrap();
                    state.resize(
                        (
                            (logical_width * factor).to_u32().unwrap(),
                            (logical_height * factor).to_u32().unwrap(),
                        ),
                        Some(factor),
                    );
                    let window_size =
                        Size::Logical(LogicalSize::new(logical_width, logical_height));
                    window.set_inner_size(window_size);
                }
                UserEvent::Quit => {
                    *control_flow = ControlFlow::Exit;
                    info!("Goodbye.");
                }
            },
            _ => {}
        }
    });
}
