use crate::events::MoyuEvent;
use crate::traits::Event;
use crate::utils::convert::to_js;

#[cfg(all(native, feature = "js_runtime"))]
pub fn dispatch_event<T: Event>(event: T) {
    use moyu_runtime::try_get_vm;

    if let Some(vm) = try_get_vm() {
        let event = MoyuEvent::from_event(event);

        let dispatch = move |vm: &moyu_runtime::QuickVM| {
            let js_event = to_js(&event).unwrap();
            if let Err(err) = vm.call_function_direct("__moyu_receive_event", vec![js_event]) {
                log::error!("failed to dispatch event: {}", err);
            }
        };

        if vm.is_vm_thread() {
            dispatch(vm);
        } else {
            vm.on_vm_thread(move |vm| {
                dispatch(vm);
            });
        }
    }
}

#[cfg(web)]
pub fn dispatch_event<T: Event>(event: T) {
    use wasm_bindgen::JsCast;
    use web_sys::js_sys::Function;

    let window = web_sys::window().unwrap();
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
}
