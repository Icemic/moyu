mod core;
mod hai;
mod nodes;
mod ops;
mod presets;
mod renderer;
mod resource;
mod surface;
mod traits;
mod types;
mod user_event;
mod utils;

use hai::create_hai_core;
#[cfg(not(feature = "web"))]
use hai_js_runtime::JSRuntime;
use hai_pal::sync::Mutex;
use hai_pal::{env, logger, platform};
use log::info;
use std::sync::Arc;
#[cfg(not(feature = "web"))]
use std::thread;
use surface::{create_wgpu_surface, create_window};
use types::SurfaceSize;
use user_event::UserEvent;
#[cfg(feature = "web")]
use wasm_bindgen::prelude::wasm_bindgen;
use winit::{
    dpi::{LogicalSize, Size},
    event::*,
    event_loop::ControlFlow,
};

fn main() {
    env::setup();
    logger::setup();
    platform::setup();

    let (event_loop, window) = create_window();

    // create event proxy which allow us to send window events from another thread
    let event_proxy = event_loop.create_proxy();
    let event_proxy = Arc::new(Mutex::new(event_proxy));

    let (surface, device, queue, config) = create_wgpu_surface(&window);

    let core = create_hai_core(surface, device, queue, config, &window, event_proxy);

    // desktop targets only
    // spawn a v8 thread
    #[cfg(not(feature = "web"))]
    {
        use log::error;
        use std::process::exit;

        let core = core.clone();

        thread::spawn(|| {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            let resource_manager = core.resource_manager.clone();
            runtime.block_on(async {
                let mut vm = JSRuntime::new(core);

                vm.with_global(|scope, global| {
                    ops::init(scope, global);
                });

                if let Err(err) = vm.prepare_entry().await {
                    error!("{}", err.to_string());
                    exit(-1);
                };

                vm.run_event_loop(|cx| {
                    let mut resource_manager = resource_manager.lock();
                    resource_manager.poll(cx)
                })
                .await;
            });
        });
    }

    #[cfg(feature = "web")]
    {
        use log::debug;
        wasm_bindgen_futures::spawn_local(async move {
            debug!("Injecting entry script.");
            let window = web_sys::window().expect("Cannot get global `window` object.");
            let document = window.document().expect("No document found.");
            let body = document.body().expect("No body found.");

            let root_script = document
                .create_element("script")
                .expect("Cannot create script element.");
            root_script
                .set_attribute("src", env::entry_dir().as_str())
                .unwrap();
            root_script.set_attribute("type", "module").unwrap();

            body.append_child(&root_script).unwrap();
        });
    }

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
#[cfg_attr(feature = "web", wasm_bindgen)]
pub fn wasm_start() {
    main();
}
