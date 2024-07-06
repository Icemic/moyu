use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "kind")]
pub enum HaiEventKind {
    MouseEnter,
    MouseLeave,
    MouseDown,
    MouseUp,
    MouseMove,
    MouseWheel,
    Click,
    KeyDown,
    KeyUp,
    KeyPress,
    TouchStart,
    TouchMove,
    TouchEnd,
    TouchCancel,
    Focus,
    Blur,
    Resize,
    Scroll,
    ContextMenu,
    FullScreenChange,
    PointerLockChange,

    // others
    NodeDestroyed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaiEvent {
    #[serde(flatten)]
    pub kind: HaiEventKind,
    pub target_id: u32,
    pub bubble_target_ids: Vec<u32>,
}

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
pub fn dispatch_event(event: HaiEvent) {
    use hai_runtime::quickjs_rusty::{owned, OwnedJsValue};
    use hai_runtime::try_get_vm;

    hai_pal::task::get_runtime_handle().spawn(async move {
        if let Some(vm) = try_get_vm() {
            let context = vm.context().context_raw();
            if let Err(err) = vm
                .call_function(
                    "__hai_receive_event",
                    vec![
                        owned!(context, format!("{:?}", event.kind)),
                        owned!(context, event.target_id),
                        owned!(context, event.bubble_target_ids),
                    ],
                )
                .await
            {
                log::error!("failed to dispatch event: {:?}", err);
            }
        }
    });
}

#[cfg(feature = "web")]
pub fn dispatch_event(event: HaiEvent) {
    use wasm_bindgen::JsCast;
    use web_sys::js_sys::Function;

    use crate::utils::convert::to_js_web;

    let window = web_sys::window().unwrap();
    if let Some(__hai_receive_event) = window.get("__hai_receive_event") {
        if __hai_receive_event.is_function() {
            let __hai_receive_event = __hai_receive_event.unchecked_ref::<Function>();
            let kind = to_js_web(&event.kind).unwrap();
            let target_id = to_js_web(&event.target_id).unwrap();
            let bubble_target_ids = to_js_web(&event.bubble_target_ids).unwrap();
            __hai_receive_event
                .call3(&window, &kind, &target_id, &bubble_target_ids)
                .unwrap();
        }
    };
}
