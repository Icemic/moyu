use crate::events::MoyuEvent;
use crate::traits::Event;
use crate::utils::convert::to_js;

pub fn dispatch_event<T: Event>(event: T) {
    dispatch_event_with_config(event, true);
}

pub fn dispatch_event_async<T: Event>(event: T) {
    dispatch_event_with_config(event, false);
}

#[cfg(all(native, feature = "js_runtime"))]
pub fn dispatch_event_with_config<T: Event>(event: T, try_sync: bool) {
    use moyu_runtime::try_get_vm;

    if let Some(vm) = try_get_vm() {
        let event = MoyuEvent::from_event(event);

        let dispatch = move |vm: &moyu_runtime::QuickVM| {
            let js_event = to_js(&event).unwrap();
            if let Err(err) = vm.call_function_direct("__moyu_receive_event", vec![js_event]) {
                log::error!("failed to dispatch event: {}", err);
            }
        };

        if try_sync && vm.is_vm_thread() {
            dispatch(vm);
        } else {
            vm.on_vm_thread(move |vm| {
                dispatch(vm);
            });
        }
    }
}

#[cfg(web)]
pub fn dispatch_event_with_config<T: Event>(event: T, try_sync: bool) {
    use wasm_bindgen::JsCast;
    use web_sys::js_sys::Function;

    let window = web_sys::window().unwrap();
    let dispatch = move || {
        if let Some(__moyu_receive_event) = window.get("__moyu_receive_event") {
            if __moyu_receive_event.is_function() {
                let __moyu_receive_event = __moyu_receive_event.unchecked_ref::<Function>();
                let event = MoyuEvent::from_event(event);
                let event = match to_js(&event) {
                    Ok(event) => event,
                    Err(err) => {
                        log::error!("failed to convert event to JS: {:?}", err);
                        return;
                    }
                };

                if let Err(err) = __moyu_receive_event.call1(&window, &event) {
                    log::error!("failed to dispatch event: {:?}", err);
                }
            }
        };
    };

    if try_sync {
        dispatch();
    } else {
        let closure = wasm_bindgen::prelude::Closure::once_into_js(dispatch);
        let window = web_sys::window().unwrap();
        let _ = window.set_timeout_with_callback(closure.as_ref().unchecked_ref());
    }
}
