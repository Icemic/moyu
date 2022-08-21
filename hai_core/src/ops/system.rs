use hai_js_runtime::{prelude::*, *};
use log::warn;
use std::{cell::RefCell, rc::Rc};

use crate::{presets::add_preset_default, state::State, user_event::UserEvent};

pub fn load_preset(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let key = (0 as u32).into_v8(scope).into();
    let preset_name = args.get(scope, key);

    if preset_name.is_none() {
        warn!("no preset name was specified, ignored.");
        return;
    }

    let preset_name = try_from_value_or_throw_exception!(scope, String, preset_name.unwrap());
    let preset_name = preset_name.to_rust_string_lossy(scope);

    match preset_name.as_str() {
        "default" => {
            let state = get_shared_state!(scope, State);
            add_preset_default(&state);
        }
        _ => {
            warn!("Unknown preset name '{}'", preset_name);
        }
    }
}

pub fn resize_window(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let state = get_shared_state!(scope, State);
    let state = state.lock().unwrap();

    let width = get_from_v8_array!(scope, args, 0);
    let height = get_from_v8_array!(scope, args, 1);
    let factor = get_from_v8_array!(scope, args, 2);

    check_exist!(scope, width);
    check_exist!(scope, height);

    let width = try_from_value_or_throw_exception!(scope, Number, width.unwrap());
    let height = try_from_value_or_throw_exception!(scope, Number, height.unwrap());
    let factor = try_from_option_value_or_throw_exception!(scope, Number, factor.unwrap());

    state
        .event_proxy
        .send_event(UserEvent::ResizeWindow(
            width.value(),
            height.value(),
            factor.and_then(|v| Some(v.value())),
        ))
        .unwrap();
}
