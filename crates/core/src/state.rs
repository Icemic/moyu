use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::utils::hit_test::HitTestTarget;

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PointerLocation {
    #[ts(skip)]
    pub valid: bool,
    pub client_x: i32,
    pub client_y: i32,
    pub screen_x: i32,
    pub screen_y: i32,
    pub offset_x: f32,
    pub offset_y: f32,
}

/// Struct for storing the state of a pointer device state
#[derive(Debug, Default, PartialEq)]
pub(crate) struct PointerState {
    /// the device type of the current event
    pub device_type: DeviceType,
    /// the location of the current event
    pub location: PointerLocation,
    /// record the current target, which is the result of hit test from current pointer location
    pub current_target: Option<HitTestTarget>,
    /// if the pointer is down (at MouseDown or TouchStart event), record the initial node id
    pub down_id: Option<u32>,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub(crate) enum DeviceType {
    #[default]
    Mouse,
    // identifier
    Finger(u32),
    Stylus,
}

pub(crate) const MOUSE_IDENTIFIER: i32 = -1;
