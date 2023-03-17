use std::sync::Arc;

use hai_core::surface::{create_wgpu_surface, create_window};
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
        let (_control_flow,) = core.handle_events(event, &window);
        if _control_flow.is_some() {
            *control_flow = _control_flow.unwrap();
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
