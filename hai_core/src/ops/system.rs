#[cfg(not(target_arch = "wasm32"))]
use hai_js_runtime::{prelude::*, *};
use log::warn;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::core::get_core;
use crate::{presets::add_preset_default, user_event::UserEvent};

#[cfg(not(target_arch = "wasm32"))]
pub fn load_preset(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let key = (0 as u32).into_v8(scope).into();
    let preset_name = args.get(scope, key);

    if preset_name.is_none() {
        warn!("no preset name was specified, ignored.");
        return;
    }

    let preset_name = try_from_value_or_throw_exception!(scope, String, preset_name.unwrap());
    let preset_name = preset_name.to_rust_string_lossy(scope);

    load_preset_inner(preset_name);
}

#[wasm_bindgen(js_name=loadPreset)]
#[cfg(target_arch = "wasm32")]
pub fn load_preset(preset_name: String) {
    load_preset_inner(preset_name);
}

pub fn load_preset_inner(preset_name: std::string::String) {
    match preset_name.as_str() {
        "default" => {
            let core = get_core();
            add_preset_default(&core);
        }
        _ => {
            warn!("Unknown preset name '{}'", preset_name);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resize_window(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let (width, height, factor) = {
        let width = get_from_v8_array!(scope, args, 0);
        let height = get_from_v8_array!(scope, args, 1);
        let factor = get_from_v8_array!(scope, args, 2);

        check_exist!(scope, width);
        check_exist!(scope, height);

        let width = try_from_value_or_throw_exception!(scope, Number, width).value();
        let height = try_from_value_or_throw_exception!(scope, Number, height).value();
        let factor = try_from_option_value_or_throw_exception!(scope, Number, factor)
            .and_then(|v| Some(v.value()));

        (width, height, factor)
    };

    resize_window_inner(width, height, factor);
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn resize_window(width: f64, height: f64, factor: Option<f64>) {
    resize_window_inner(width, height, factor);
}

pub fn resize_window_inner(width: f64, height: f64, factor: Option<f64>) {
    let core = get_core();
    core
        .event_proxy
        .lock()
        .send_event(UserEvent::ResizeWindow(width, height, factor))
        .unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn quit(_: &mut HandleScope, _: Local<Array>, _: Option<Local<Function>>) {
    quit_inner();
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn quit() {
    quit_inner();
}

pub fn quit_inner() {
    let core = get_core();
    core
        .event_proxy
        .lock()
        .send_event(UserEvent::Quit)
        .unwrap();
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn load_resources() {
    use futures::future::poll_fn;

    wasm_bindgen_futures::spawn_local(async {
        let resource_manager = {
            let core = get_core();
            core.resource_manager.clone()
        };
        let mut resource_manager = resource_manager.lock().unwrap();
        poll_fn(|cx| resource_manager.poll(cx)).await;
    });
}
