use std::sync::Arc;
use winit::event::Event;

use hai_core::surface::{create_eventloop, create_wgpu_surface, create_window};
use hai_core::{create_hai_core, setup, spawn_runtime_with_core};
use hai_pal::{env, logger, platform};

fn main() {
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

    event_loop.run(move |event, event_loop, control_flow| {
        match event {
            Event::Resumed => {
                let _window = create_window(&event_loop);

                let (surface, device, queue, config) = create_wgpu_surface(&_window);

                let _core = create_hai_core(
                    surface,
                    device,
                    queue,
                    config,
                    &_window,
                    event_proxy.clone(),
                );

                spawn_runtime_with_core(&_core, None);

                _window.set_visible(true);

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
                let (_control_flow,) = core.handle_events(&event, &window);
                if _control_flow.is_some() {
                    *control_flow = _control_flow.unwrap();
                }
            }
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
