use std::time::Duration;

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

/// A mouse press remembered until its button is released, so the release can be
/// paired with it and turned into a click gesture.
#[derive(Debug, PartialEq)]
pub(crate) struct MousePress {
    /// Node that was under the pointer when the button went down.
    pub node_id: u32,
    /// Ancestors of that node, ordered root first and excluding the root.
    pub parent_ids: Vec<u32>,
}

/// A touch that is still a candidate for the mouse compatibility gesture. Tracked from
/// `Start` until `End` / `Cancel`, so the release can tell a tap from a drag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TapGesture {
    /// Where the touch started, in stage logical coordinates.
    pub start_x: f32,
    pub start_y: f32,
    /// Set once the touch travels past the tap slop, or when another touch becomes
    /// active.
    pub cancelled: bool,
}

/// The last completed left click, used to recognize a double click.
#[derive(Debug, PartialEq)]
pub(crate) struct ClickRecord {
    /// Gesture target of the click, the nearest common ancestor of its press and release.
    pub target_id: u32,
    /// Stage logical position of the click.
    pub x: f32,
    pub y: f32,
    /// Engine uptime when the click completed.
    pub at: Duration,
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
    /// if a touch is down (started and not yet ended), record the node id it started on
    pub touch_down_id: Option<u32>,
    /// tap candidate for the touch currently down on this pointer, if any
    pub tap: Option<TapGesture>,
    /// press records per mouse button, indexed by `PointerButton::index()`
    pub mouse_downs: [Option<MousePress>; 5],
    /// the last completed left click, for double click detection
    pub last_click: Option<ClickRecord>,
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
