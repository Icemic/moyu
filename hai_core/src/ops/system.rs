#[cfg(not(feature = "web"))]
use hai_js_runtime::{prelude::*, *};
#[cfg(not(feature = "web"))]
use hai_macros::hai_bindgen;
use log::warn;
#[cfg(feature = "web")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::core::get_core;
#[cfg(not(feature = "web"))]
use crate::utils::convert::from_js;
use crate::{presets::add_preset_default, user_event::UserEvent};

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn load_preset(preset_name: std::string::String) -> Result<(), std::string::String> {
    match preset_name.as_str() {
        "default" => {
            let core = get_core();
            add_preset_default(&core);
        }
        _ => {
            warn!("Unknown preset name '{}'", preset_name);
        }
    }
    Ok(())
}

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn resize_window(
    width: f64,
    height: f64,
    factor: Option<f64>,
) -> Result<(), std::string::String> {
    let core = get_core();
    core.event_proxy
        .lock()
        .send_event(UserEvent::ResizeWindow(width, height, factor))
        .unwrap();
    Ok(())
}

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn quit() -> Result<(), std::string::String> {
    let core = get_core();
    core.event_proxy.lock().send_event(UserEvent::Quit).unwrap();
    Ok(())
}

#[wasm_bindgen]
#[cfg(feature = "web")]
pub fn load_resources() {
    use futures::future::poll_fn;

    wasm_bindgen_futures::spawn_local(async {
        let resource_manager = {
            let core = get_core();
            core.resource_manager.clone()
        };
        let mut resource_manager = resource_manager.lock();
        poll_fn(|cx| resource_manager.poll(cx)).await;
    });
}
