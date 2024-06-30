use anyhow::Result;
#[cfg(not(feature = "web"))]
use hai_macros::hai_bindgen;
#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
use hai_runtime::quickjs_rusty::{JSContext, RawJSValue};
#[cfg(feature = "web")]
use wasm_bindgen::prelude::wasm_bindgen;

use hai_core::core::get_core;
use hai_core::user_event::UserEvent;
use hai_core::user_event::WindowState;
#[cfg(not(feature = "web"))]
use hai_core::utils::convert::{from_js, JSValue};

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn resize_window(
    width: f64,
    height: f64,
    factor: Option<f64>,
) -> Result<(), std::string::String> {
    let core = get_core();
    core.send_event(UserEvent::ResizeWindow(width, height, factor));
    Ok(())
}

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn set_idle() -> Result<(), std::string::String> {
    let core = get_core();
    core.send_event(UserEvent::WindowState(WindowState::Idle));
    Ok(())
}

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn set_fullscreen() -> Result<(), std::string::String> {
    let core = get_core();
    core.send_event(UserEvent::WindowState(WindowState::Fullscreen));
    Ok(())
}

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn set_maximized() -> Result<(), std::string::String> {
    let core = get_core();
    core.send_event(UserEvent::WindowState(WindowState::Maximized));
    Ok(())
}

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn set_minimized() -> Result<(), std::string::String> {
    let core = get_core();
    core.send_event(UserEvent::WindowState(WindowState::Minimized));
    Ok(())
}

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn quit() -> Result<(), std::string::String> {
    let core = get_core();
    core.send_event(UserEvent::Quit);
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
