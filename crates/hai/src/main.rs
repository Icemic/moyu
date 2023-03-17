use log::info;
use std::sync::Arc;

use winit::{
    dpi::{LogicalSize, Size},
    event::*,
    event_loop::ControlFlow,
};

use hai_core::surface::{create_wgpu_surface, create_window};
use hai_core::types::SurfaceSize;
use hai_core::user_event::UserEvent;
use hai_core::{create_hai_core, setup, spawn_runtime_with_core};
use hai_pal::sync::Mutex;
use hai_pal::{env, logger, platform};

fn main() {
    env::setup();
    logger::setup();
    platform::setup();

    setup();

    let (event_loop, window) = create_window();

    // create event proxy which allow us to send window events from another thread
    let event_proxy = event_loop.create_proxy();
    let event_proxy = Arc::new(Mutex::new(event_proxy));

    let (surface, device, queue, config) = create_wgpu_surface(&window);

    let core = create_hai_core(surface, device, queue, config, &window, event_proxy);

    spawn_runtime_with_core(&core);

    window.set_visible(true);

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                match core.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => {
                        core.refresh();
                    }
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
                if !core.input(event) {
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
                            let surface_size = SurfaceSize::from_physical_size(
                                physical_size,
                                window.scale_factor(),
                            );
                            core.resize(surface_size);
                        }
                        WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            new_inner_size,
                            ..
                        } => {
                            let surface_size = SurfaceSize::from_physical_size(
                                new_inner_size.to_owned(),
                                scale_factor.clone(),
                            );
                            core.resize(surface_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::UserEvent(user_event) => match user_event {
                UserEvent::ResizeWindow(logical_width, logical_height, factor) => {
                    let factor = factor.unwrap_or(window.scale_factor());

                    if logical_width > 0. && logical_height > 0. {
                        let surface_size = SurfaceSize::new(logical_width, logical_height, factor);
                        core.resize(surface_size);

                        let window_size =
                            Size::Logical(LogicalSize::new(logical_width, logical_height));
                        window.set_inner_size(window_size);
                    }
                }
                UserEvent::SetTitle(title) => {
                    window.set_title(&title);
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

#[cfg(feature = "web")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(feature = "web")]
#[cfg_attr(feature = "web", wasm_bindgen)]
pub fn wasm_start() {
    main();
}
