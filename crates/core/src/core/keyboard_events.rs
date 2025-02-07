use std::sync::Arc;

use winit::event::WindowEvent;
use winit::keyboard::NamedKey;
use winit::window::Window;

use crate::events::{KeyboardEvent, KeyboardEventKind, KeyboardLocation};
use crate::utils::dispatch_event::dispatch_event;

use super::Core;

impl Core {
    pub fn handle_keyboard_events(&self, _window: &Window, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic,
            } => {
                if !*is_synthetic {
                    let modifiers_state = self.modifiers_state.load().clone();

                    let kind = if event.state.is_pressed() {
                        KeyboardEventKind::KeyDown
                    } else {
                        KeyboardEventKind::KeyUp
                    };

                    let location = match event.location {
                        winit::keyboard::KeyLocation::Standard => KeyboardLocation::Standard,
                        winit::keyboard::KeyLocation::Left => KeyboardLocation::Left,
                        winit::keyboard::KeyLocation::Right => KeyboardLocation::Right,
                        winit::keyboard::KeyLocation::Numpad => KeyboardLocation::Numpad,
                    };

                    let key = match event.logical_key.clone() {
                        winit::keyboard::Key::Named(named_key) => {
                            if named_key == NamedKey::Space {
                                " ".to_string()
                            } else {
                                format!("{:?}", named_key)
                            }
                        }
                        winit::keyboard::Key::Character(c) => c.to_string(),
                        winit::keyboard::Key::Unidentified(native_key) => {
                            log::warn!("Unidentified key {:?} is not supported", native_key);
                            "Unidentified".to_string()
                        }
                        winit::keyboard::Key::Dead(_) => {
                            log::warn!("Dead key is not supported");
                            "".to_string()
                        }
                    };

                    let code = match event.physical_key {
                        winit::keyboard::PhysicalKey::Code(key_code) => format!("{:?}", key_code),
                        winit::keyboard::PhysicalKey::Unidentified(native_key_code) => {
                            log::warn!(
                                "Unidentified key code {:?} is not supported",
                                native_key_code
                            );
                            "Unidentified".to_string()
                        }
                    };

                    let event = KeyboardEvent {
                        kind,
                        target_id: 0,
                        bubble_target_ids: vec![],
                        key,
                        code,
                        location,
                        repeat: event.repeat,
                        ctrl_key: modifiers_state.control_key(),
                        shift_key: modifiers_state.shift_key(),
                        alt_key: modifiers_state.alt_key(),
                        meta_key: modifiers_state.super_key(),
                        // is_composing should always be false since KeyboardInput event will not
                        // be fired when the user is composing text. (IME event will be fired instead)
                        is_composing: false,
                    };

                    dispatch_event(event);
                }

                true
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers_state.store(Arc::new(modifiers.state()));
                true
            }
            _ => false,
        }
    }
}
