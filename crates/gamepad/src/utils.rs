use gilrs::{Axis, Button};

pub(crate) fn get_w3c_button(button: Button) -> u32 {
    match button {
        Button::South => 0,
        Button::East => 1,
        Button::West => 2,
        Button::North => 3,
        Button::LeftTrigger => 4,
        Button::RightTrigger => 5,
        Button::LeftTrigger2 => 6,
        Button::RightTrigger2 => 7,
        Button::Select => 8,
        Button::Start => 9,
        Button::LeftThumb => 10,
        Button::RightThumb => 11,
        Button::DPadUp => 12,
        Button::DPadDown => 13,
        Button::DPadLeft => 14,
        Button::DPadRight => 15,
        Button::Mode => 16,
        Button::C | Button::Z | Button::Unknown => {
            log::warn!("Button {:?} is not a valid W3C button", button);
            255
        }
    }
}

pub(crate) fn get_w3c_axis(axis: Axis) -> u32 {
    match axis {
        Axis::LeftStickX => 0,
        Axis::LeftStickY => 1,
        Axis::RightStickX => 2,
        Axis::RightStickY => 3,
        Axis::LeftZ | Axis::RightZ | Axis::DPadX | Axis::DPadY | Axis::Unknown => {
            log::warn!("Axis {:?} is not a valid W3C axis", axis);
            255
        }
    }
}
