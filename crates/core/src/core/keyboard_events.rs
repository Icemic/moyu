use std::sync::Arc;

use winit::event::WindowEvent;
use winit::keyboard::NamedKey;
use winit::window::Window;

#[cfg(android)]
use crate::events::BeforeUnloadEvent;
use crate::events::{KeyboardEvent, KeyboardEventKind, KeyboardLocation};
use crate::utils::dispatch_event::dispatch_event;

use super::Core;
use super::input::{KeyInput, KeyboardModifiers};

impl Core {
    pub fn handle_keyboard_events(&self, _window: &Window, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic,
            } => {
                if !*is_synthetic {
                    #[cfg(android)]
                    if event.state.is_pressed()
                        && matches!(
                            &event.logical_key,
                            winit::keyboard::Key::Named(NamedKey::BrowserBack)
                        )
                    {
                        dispatch_event(BeforeUnloadEvent {});
                        return true;
                    }

                    let modifiers_state = self.modifiers_state.load().clone();

                    self.editable.handle_keyboard_input(event, *modifiers_state);

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

                    dispatch_keyboard_event(KeyInput {
                        kind,
                        key,
                        code,
                        location,
                        repeat: event.repeat,
                        modifiers: KeyboardModifiers {
                            ctrl: modifiers_state.control_key(),
                            shift: modifiers_state.shift_key(),
                            alt: modifiers_state.alt_key(),
                            meta: modifiers_state.super_key(),
                        },
                    });
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

/// Dispatch one keyboard event to the engine.
///
/// Shared by the window event path and synthetic input.
pub(super) fn dispatch_keyboard_event(input: KeyInput) {
    dispatch_event(KeyboardEvent {
        kind: input.kind,
        target_id: 0,
        bubble_target_ids: vec![],
        key: input.key,
        code: input.code,
        location: input.location,
        repeat: input.repeat,
        ctrl_key: input.modifiers.ctrl,
        shift_key: input.modifiers.shift,
        alt_key: input.modifiers.alt,
        meta_key: input.modifiers.meta,
        // is_composing should always be false since KeyboardInput event will not
        // be fired when the user is composing text. (IME event will be fired instead)
        is_composing: false,
    });
}
