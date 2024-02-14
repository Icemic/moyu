use log::error;
use serde::{Deserialize, Serialize};

use hai_pal::task::get_runtime_handle;

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "v8"))]
use hai_js_runtime::get_vm;
#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
use hai_runtime::get_vm;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaiEvent {
    pub kind: HaiEventKind,
    pub target_id: u32,
}

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
pub fn dispatch_event(event: HaiEvent) {
    get_runtime_handle().spawn(async move {
        if let Err(err) = get_vm()
            .call_function(
                "__hai_receive_event",
                vec![format!("{:?}", event.kind), event.target_id.to_string()],
            )
            .await
        {
            error!("failed to dispatch event: {:?}", err);
        }
    });
}

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "v8"))]
pub fn dispatch_event(_: HaiEvent) {
    log::error!("dispatch_event is not implemented for v8 runtime, nothing will happen");
}
