#![feature(drain_filter)]

mod node;
mod ops;
mod presets;
mod renderer;
mod sprite;
mod state;
mod texture;
mod traits;
mod types;

#[cfg(not(target_arch = "wasm32"))]
use hai_js_runtime::JSRuntime;
use hai_pal::{env, logger, platform};
use presets::add_preset_default;
use renderer::Renderer;
use state::State;
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
use winit::{
    dpi::{LogicalSize, Size},
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    platform::setup();
    env::setup();
    logger::setup();

    // create multithread shared state
    let state = State::new();
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
            runtime.block_on(async {
                let mut vm = JSRuntime::new(state);
                vm.prepare_static_modules().await;

                vm.with_global(|scope, global| {
                    ops::init(scope, global);
                });

                vm.start();

                vm.run_event_loop().await;
            });
        });
    }

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
    let event_loop = EventLoop::new();
    // create window
    let window = WindowBuilder::new()
        .with_inner_size(Size::Logical(LogicalSize::new(1280., 720.)))
        .build(&event_loop)
        .unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    let mut renderer = {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(Renderer::new(&window))
    };

    #[cfg(target_arch = "wasm32")]
    let mut renderer = { pollster::block_on(Renderer::new(&window)) };

    add_preset_default(&state, &renderer);

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                renderer.update(&state);
                match renderer.render(&state) {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => renderer.refresh(),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                // makes State to have priority over main()
                if !renderer.input(event) {
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
                            renderer.resize(*physical_size, None);
                        }
                        WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            new_inner_size,
                            ..
                        } => {
                            renderer.resize(**new_inner_size, Some(*scale_factor));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
}
