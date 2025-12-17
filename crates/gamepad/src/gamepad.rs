use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The Gamepad interface of the [Gamepad API](https://developer.mozilla.org/en-US/docs/Web/API/Gamepad_API)
/// defines an individual gamepad or other controller, allowing access to information such as button presses,
/// axis positions, and id.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Gamepad {
    /// A string containing identifying information about the controller.
    pub id: String,
    /// An integer that is auto-incremented to be unique for each device currently connected to the system.
    pub index: u32,
    /// A boolean indicating whether the gamepad is still connected to the system.
    pub connected: bool,
    /// A string indicating whether the browser has remapped the controls on the device to a known layout.
    /// Currently, the only value that is returned is "standard", which indicates that the browser has
    /// remapped the controls to a standard layout.
    /// See [gamepad#remapping](https://w3c.github.io/gamepad/#remapping)
    pub mapping: String,
    /// An array of [GamepadButton] objects representing the buttons present on the device.
    pub buttons: Vec<GamepadButton>,
    /// An array representing the controls with axes present on the device (e.g. analog thumb sticks).
    pub axes: Vec<f32>,
    /// A GamepadHapticActuator object, which represents haptic feedback hardware available on the controller.
    pub vibration_actuator: Option<GamepadHapticActuator>,
    /// A f64 representing the last time the data for this gamepad was updated.
    pub timestamp: f64,
}

impl Default for Gamepad {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            index: 0,
            connected: true,
            mapping: "standard".to_string(),
            buttons: vec![GamepadButton::default(); 17],
            axes: vec![0.0; 4],
            vibration_actuator: None,
            timestamp: 0.0,
        }
    }
}

/// The GamepadButton interface defines an individual button of a gamepad or other controller, allowing
/// access to the current state of different types of buttons available on the control device.
#[derive(Debug, Default, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GamepadButton {
    /// A boolean value indicating whether the button is currently pressed (`true`) or unpressed (`false`).
    pub pressed: bool,
    /// A boolean value indicating whether the button is currently touched (`true`) or not touched (`false`).
    pub touched: bool,
    /// A double value used to represent the current state of analog buttons, such as the triggers on many
    /// modern gamepads. The values are normalized to the range 0.0 — 1.0, with 0.0 representing a button that
    /// is not pressed, and 1.0 representing a button that is fully pressed.
    pub value: f32,
}

/// The GamepadHapticActuator interface of the [Gamepad API](https://developer.mozilla.org/en-US/docs/Web/API/Gamepad_API)
/// represents hardware in the controller designed to provide haptic feedback to the user (if available),
/// most commonly vibration hardware.
#[derive(Debug, Default, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GamepadHapticActuator {
    /// Returns an array of enumerated values representing the different haptic effects that the actuator supports.
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum GamepadHapticActuatorEffect {
    /// A positional rumbling effect created by dual vibration motors in each handle of a controller,
    /// which can be vibrated independently.
    DualRumble,
    /// Localized rumbling effects on the surface of a controller's trigger buttons created by vibrational
    /// motors located in each button. These buttons most commonly take the form of spring-loaded triggers.
    TriggerRumble,
}
