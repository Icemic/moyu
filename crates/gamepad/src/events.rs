use moyu_core::traits::Event;
use serde::Serialize;
use ts_rs::TS;

use crate::{Gamepad, GamepadButton};

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", untagged)]
#[ts(export, optional_fields)]
pub enum GamepadEvent {
    Connected {
        gamepad: Gamepad,
    },
    Disconnected {
        gamepad: Gamepad,
    },
    ButtonChanged {
        gamepad: Gamepad,
        button: GamepadButton,
    },
    ButtonDown {
        gamepad: Gamepad,
        button: GamepadButton,
    },
    ButtonUp {
        gamepad: Gamepad,
        button: GamepadButton,
    },
    AxisChanged {
        gamepad: Gamepad,
        axis: u32,
        value: f32,
    },
}

impl Event for GamepadEvent {
    fn name(&self) -> &'static str {
        match self {
            GamepadEvent::Connected { .. } => "gamepadconnected",
            GamepadEvent::Disconnected { .. } => "gamepaddisconnected",
            GamepadEvent::ButtonChanged { .. } => "gamepadbuttonchanged",
            GamepadEvent::ButtonDown { .. } => "gamepadbuttondown",
            GamepadEvent::ButtonUp { .. } => "gamepadbuttonup",
            GamepadEvent::AxisChanged { .. } => "gamepadaxischanged",
        }
    }
}
