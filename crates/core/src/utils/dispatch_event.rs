use crate::events::HaiEvent;
use crate::traits::Event;
use crate::utils::convert::to_js;

#[cfg(all(native, feature = "js_runtime"))]
pub fn dispatch_event<T: Event>(event: T) {
    use doufu_runtime::try_get_vm;

    doufu_pal::task::get_runtime_handle().spawn(async move {
        if let Some(vm) = try_get_vm() {
            vm.with_context(move |vm| {
                let event = HaiEvent::from_event(event);
                let event = to_js(&event).unwrap();
                if let Err(err) = vm.call_function_direct("__doufu_receive_event", vec![event]) {
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
    if let Some(__doufu_receive_event) = window.get("__doufu_receive_event") {
        if __doufu_receive_event.is_function() {
            let __doufu_receive_event = __doufu_receive_event.unchecked_ref::<Function>();
            let event = HaiEvent::from_event(event);
            let event = to_js(&event).unwrap();
            __doufu_receive_event.call1(&window, &event).unwrap();
        }
    };
}
