use hai_pal::task::get_runtime_handle;
use hai_runtime::get_vm;
use log::error;
use serde::{Deserialize, Serialize};

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
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaiEvent {
    pub kind: HaiEventKind,
    pub target_id: u32,
}

pub fn dispatch_event(event: HaiEvent) {
    println!("dispatch_event: {:?}", event);

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
