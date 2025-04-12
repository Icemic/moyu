use moyu_core::traits::Event;
use serde::Serialize;

use crate::{Gamepad, GamepadButton};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", untagged)]
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
    AxisChanged {
        gamepad: Gamepad,
        axis: u32,
        value: f32,
    },
}

impl Event for GamepadEvent {
    fn name(&self) -> &'static str {
        "gamepad"
    }
}
