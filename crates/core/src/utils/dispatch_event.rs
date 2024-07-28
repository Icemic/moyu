use serde::{Deserialize, Serialize};

use super::hit_test::HitTestTarget;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize)]
pub enum HaiEventKind {
    #[default]
    Unknown,

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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaiEvent {
    // #[serde(flatten)]
    pub kind: HaiEventKind,
    pub target_id: u32,
    pub bubble_target_ids: Vec<u32>,
    /// for mouse event and touch event,
    /// client_x, client_y, screen_x, screen_y, x, y in order
    pub location: Option<(u32, u32, u32, u32, f32, f32)>,
    /// for touch event
    pub identifier: Option<u32>,
}

/// Struct for storing the state of a pointer device state
#[derive(Debug, Default, PartialEq)]
pub struct PointerState {
    /// the device type of the current event
    pub device_type: DeviceType,
    /// the location of the current event, (client_x, client_y, screen_x, screen_y, x, y) in order
    pub location: (u32, u32, u32, u32, f32, f32),
    /// record the current target, which is the result of hit test from current pointer location
    pub current_target: Option<HitTestTarget>,
    /// if the pointer is down (at MouseDown or TouchStart event), record the initial node id
    pub down_id: Option<u32>,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub enum DeviceType {
    #[default]
    Mouse,
    // identifier
    Finger(u32),
    Stylus,
}

pub const MOUSE_IDENTIFIER: i32 = -1;

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
pub fn dispatch_event(event: HaiEvent) {
    use hai_runtime::try_get_vm;

    use crate::utils::convert::to_js;

    hai_pal::task::get_runtime_handle().spawn(async move {
        if let Some(vm) = try_get_vm() {
            vm.with_context(move |vm| {
                let event = to_js(&event).unwrap();
                if let Err(err) = vm.call_function_direct("__hai_receive_event", vec![event]) {
                    log::error!("failed to dispatch event: {:?}", err);
                }
            })
        }
    });
}

#[cfg(feature = "web")]
pub fn dispatch_event(event: HaiEvent) {
    use wasm_bindgen::JsCast;
    use web_sys::js_sys::Function;

    use crate::utils::convert::to_js;

    let window = web_sys::window().unwrap();
    if let Some(__hai_receive_event) = window.get("__hai_receive_event") {
        if __hai_receive_event.is_function() {
            let __hai_receive_event = __hai_receive_event.unchecked_ref::<Function>();
            let kind = to_js(&event.kind).unwrap();
            let target_id = to_js(&event.target_id).unwrap();
            let bubble_target_ids = to_js(&event.bubble_target_ids).unwrap();
            __hai_receive_event
                .call3(&window, &kind, &target_id, &bubble_target_ids)
                .unwrap();
        }
    };
}
