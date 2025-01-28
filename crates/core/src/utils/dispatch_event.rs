use crate::events::HaiEvent;
use crate::traits::Event;
use crate::utils::convert::to_js;

#[cfg(all(native, feature = "js_runtime"))]
pub fn dispatch_event<T: Event>(event: T) {
    use hai_runtime::try_get_vm;

    hai_pal::task::get_runtime_handle().spawn(async move {
        if let Some(vm) = try_get_vm() {
            vm.with_context(move |vm| {
                let event = HaiEvent::from_event(event);
                let event = to_js(&event).unwrap();
                if let Err(err) = vm.call_function_direct("__hai_receive_event", vec![event]) {
                    log::error!("failed to dispatch event: {:?}", err);
                }
            })
        }
    });
}

#[cfg(web)]
pub fn dispatch_event<T: Event>(event: T) {
    use wasm_bindgen::JsCast;
    use web_sys::js_sys::Function;

    let window = web_sys::window().unwrap();
    if let Some(__hai_receive_event) = window.get("__hai_receive_event") {
        if __hai_receive_event.is_function() {
            let __hai_receive_event = __hai_receive_event.unchecked_ref::<Function>();
            let event = HaiEvent::from_event(event);
            let event = to_js(&event).unwrap();
            __hai_receive_event.call1(&window, &event).unwrap();
        }
    };
}
