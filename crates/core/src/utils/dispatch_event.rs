use log::error;
use serde::{Deserialize, Serialize};

use hai_pal::task::get_runtime_handle;

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "v8"))]
use hai_js_runtime::get_vm;
#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
use hai_runtime::get_vm;

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

    get_runtime_handle().spawn(async move {
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
                error!("failed to dispatch event: {:?}", err);
            }
        }
    });
}

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "v8"))]
pub fn dispatch_event(_: HaiEvent) {
    log::error!("dispatch_event is not implemented for v8 runtime, nothing will happen");
}
