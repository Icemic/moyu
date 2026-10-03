//! The engine's own input vocabulary and the synthetic input entry points.
//!
//! Window events arrive as winit types; these types name the same concepts in engine
//! terms, so that input can be described and processed without depending on a windowing
//! backend. The window event paths in `pointer_events` and `keyboard_events` map onto
//! them at the edge and share their dispatch helpers with the synthetic entry points
//! below, which the debug bridge exposes over `engine:mouse` / `engine:touch` /
//! `engine:key`.
//!
//! Synthetic input drives that same pipeline instead of faking events: it writes pointer
//! state, runs hit testing and dispatches the engine events a window event would, so an
//! injected click has the same effects as a user's.

use std::sync::Arc;

use winit::keyboard::ModifiersState;

use crate::events::{KeyboardEventKind, KeyboardLocation, WheelEventDeltaMode};
use crate::state::{DeviceType, MOUSE_IDENTIFIER, PointerLocation};

use super::Core;

/// Mouse button an action uses. Only the buttons DOM defines are carried; physical
/// buttons beyond them are ignored by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Left,
    Middle,
    Right,
    Back,
    Forward,
}

impl PointerButton {
    /// All buttons, in the order their press records occupy in
    /// [`crate::state::PointerState::mouse_downs`].
    pub(crate) const ALL: [Self; 5] = [
        Self::Left,
        Self::Middle,
        Self::Right,
        Self::Back,
        Self::Forward,
    ];

    /// Index this button's press record occupies in `PointerState::mouse_downs`.
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Middle => 1,
            Self::Right => 2,
            Self::Back => 3,
            Self::Forward => 4,
        }
    }

    /// The button's bit in the DOM `MouseEvent.buttons` mask. The auxiliary (middle)
    /// button is 4, not 2.
    pub(crate) fn bit(self) -> u32 {
        match self {
            Self::Left => 1,
            Self::Middle => 4,
            Self::Right => 2,
            Self::Back => 8,
            Self::Forward => 16,
        }
    }

    /// The button's value in the DOM `MouseEvent.button` property.
    pub(crate) fn as_button_number(self) -> i32 {
        match self {
            Self::Left => 0,
            Self::Middle => 1,
            Self::Right => 2,
            Self::Back => 3,
            Self::Forward => 4,
        }
    }
}

/// Phase of a touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

/// What to do with the pointer at a position on the stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointerAction {
    Move,
    /// Press the button. The press records the target the release pairs with, so the
    /// released button is what completes the gesture.
    Down(PointerButton),
    Up(PointerButton),
    /// Press and release in one action, which is what a window reports as a click.
    Click(PointerButton),
    Wheel {
        delta_x: f64,
        delta_y: f64,
        mode: WheelEventDeltaMode,
    },
}

/// Modifier keys held while a keyboard event is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

/// One keyboard event to send.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyInput {
    pub kind: KeyboardEventKind,
    pub key: String,
    pub code: String,
    pub location: KeyboardLocation,
    pub repeat: bool,
    pub modifiers: KeyboardModifiers,
}

/// What a synthetic pointer action reached.
#[derive(Debug, Clone, PartialEq)]
pub struct PointerInputReport {
    /// Node the action targeted, as decided by hit testing. The root node (id 0) when
    /// nothing interactive was hit.
    pub target_node_id: u32,
    /// Ancestors of the target, in the order events bubble through them.
    pub bubble_node_ids: Vec<u32>,
    /// Where the pointer ended up, in stage logical coordinates.
    pub location: PointerLocation,
    /// Event kinds the engine dispatched, named as JavaScript sees them ("MouseDown").
    pub dispatched: Vec<&'static str>,
}

/// Why a touch phase could not be processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    /// A phase other than `Start` arrived for a touch that was never started.
    NoActiveTouch(u32),
    /// A `Start` phase arrived without a position.
    MissingPosition,
}

/// Where the mouse events one dispatch reports came from.
///
/// The engine dispatches some mouse events on its own: the mouse compatibility gesture of
/// a touch tap, and the hover refresh a frame performs while the pointer stays where it
/// is. Those are marked [`MouseEventSource::Synthetic`] so a consumer can tell them from
/// the user operating a mouse; a synthetic input call such as the debug bridge's is a
/// user's gesture as far as the engine is concerned, so it reports
/// [`MouseEventSource::User`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MouseEventSource {
    /// A pointer device the user operated.
    User,
    /// The engine itself.
    Synthetic,
}

/// Events one input action dispatched, for the caller to report.
///
/// Recording is opt-in: the window event path passes [`DispatchRecord::disabled`], so its
/// hot path stays allocation-free, while synthetic input collects the names.
pub(crate) struct DispatchRecord<'a> {
    entries: Option<&'a mut Vec<&'static str>>,
}

impl<'a> DispatchRecord<'a> {
    /// A record that drops everything.
    pub(crate) fn disabled() -> Self {
        Self { entries: None }
    }

    /// A record that appends dispatched event kinds to `entries`.
    pub(crate) fn enabled(entries: &'a mut Vec<&'static str>) -> Self {
        Self {
            entries: Some(entries),
        }
    }

    pub(crate) fn push(&mut self, name: &'static str) {
        if let Some(entries) = self.entries.as_deref_mut() {
            entries.push(name);
        }
    }
}

impl Core {
    /// Send one pointer action to a position on the stage.
    ///
    /// The pointer moves there first, so a single `Click` works without a preceding
    /// move, and the action sees the same hover state a real pointer would have.
    pub fn simulate_pointer(&self, x: f32, y: f32, action: PointerAction) -> PointerInputReport {
        let mut dispatched = Vec::new();
        let mut record = DispatchRecord::enabled(&mut dispatched);

        self.pointer_to(MOUSE_IDENTIFIER, x, y);
        self.handle_pointer_hover(MOUSE_IDENTIFIER, true, MouseEventSource::User, &mut record);

        match action {
            PointerAction::Move => {}
            PointerAction::Down(button) => self.pointer_press(
                MOUSE_IDENTIFIER,
                button,
                MouseEventSource::User,
                &mut record,
            ),
            PointerAction::Up(button) => self.pointer_release(
                MOUSE_IDENTIFIER,
                button,
                MouseEventSource::User,
                &mut record,
            ),
            PointerAction::Click(button) => {
                self.pointer_press(
                    MOUSE_IDENTIFIER,
                    button,
                    MouseEventSource::User,
                    &mut record,
                );
                self.pointer_release(
                    MOUSE_IDENTIFIER,
                    button,
                    MouseEventSource::User,
                    &mut record,
                );
            }
            PointerAction::Wheel {
                delta_x,
                delta_y,
                mode,
            } => self.pointer_wheel(MOUSE_IDENTIFIER, delta_x, delta_y, mode, &mut record),
        }

        self.pointer_report(MOUSE_IDENTIFIER, dispatched)
    }

    /// Advance one touch.
    ///
    /// The position is required for `Start`; later phases may omit it and keep the
    /// touch's current position. A phase other than `Start` needs a touch that is
    /// currently active — started and not yet ended — which is the order a real device
    /// produces.
    pub fn simulate_touch(
        &self,
        phase: TouchPhase,
        x: Option<f32>,
        y: Option<f32>,
        identifier: u32,
    ) -> Result<PointerInputReport, InputError> {
        let pointer_id = identifier as i32;
        let previous = self
            .pointer_map
            .get(&pointer_id)
            .map(|state| state.location);

        // A press marks a touch as active: `Start` sets it and `End` / `Cancel` clear
        // it, so a touch that already ended does not pass as active again.
        let active = self
            .pointer_map
            .get(&pointer_id)
            .is_some_and(|state| state.touch_down_id.is_some());

        match phase {
            TouchPhase::Start => {
                let (Some(x), Some(y)) = (x, y) else {
                    return Err(InputError::MissingPosition);
                };

                self.get_ensure_pointer_state(pointer_id, DeviceType::Finger(identifier));
                self.pointer_to(pointer_id, x, y);
            }
            TouchPhase::Move | TouchPhase::End | TouchPhase::Cancel => {
                if !active {
                    return Err(InputError::NoActiveTouch(identifier));
                }

                if let (Some(x), Some(y)) = (x, y) {
                    self.pointer_to(pointer_id, x, y);
                }
            }
        }

        let mut dispatched = Vec::new();
        let mut record = DispatchRecord::enabled(&mut dispatched);

        // A real device refreshes what a touch hovers when it starts, and keeps
        // targeting the same node while it moves.
        self.handle_pointer_hover(
            pointer_id,
            phase == TouchPhase::Start,
            MouseEventSource::User,
            &mut record,
        );

        // A move that did not change the position is a duplicated event, which the
        // window event path drops as well.
        let unchanged = self
            .pointer_map
            .get(&pointer_id)
            .is_some_and(|state| Some(state.location) == previous);

        if !(phase == TouchPhase::Move && unchanged) {
            self.pointer_touch(pointer_id, phase, &mut record);
        }

        Ok(self.pointer_report(pointer_id, dispatched))
    }

    /// Send one keyboard event.
    ///
    /// Keyboard events go to the engine as a whole (no target node), exactly as the
    /// window event path sends them. The injected modifiers also become the shared
    /// modifier state, so a synthetic mouse event right after carries them the same
    /// way it would after a real key press.
    pub fn simulate_key(&self, input: KeyInput) {
        let mut modifiers = ModifiersState::empty();
        if input.modifiers.ctrl {
            modifiers |= ModifiersState::CONTROL;
        }
        if input.modifiers.shift {
            modifiers |= ModifiersState::SHIFT;
        }
        if input.modifiers.alt {
            modifiers |= ModifiersState::ALT;
        }
        if input.modifiers.meta {
            modifiers |= ModifiersState::SUPER;
        }
        self.modifiers_state.store(Arc::new(modifiers));

        super::keyboard_events::dispatch_keyboard_event(input);
    }

    /// Describe what a pointer action ended up targeting.
    fn pointer_report(&self, identifier: i32, dispatched: Vec<&'static str>) -> PointerInputReport {
        let Some(state) = self.pointer_map.get(&identifier) else {
            return PointerInputReport {
                target_node_id: 0,
                bubble_node_ids: Vec::new(),
                location: PointerLocation::default(),
                dispatched,
            };
        };

        let (target_node_id, bubble_node_ids) = match &state.current_target {
            Some(target) => (*target.node.read().base().id(), target.parent_ids.clone()),
            None => (0, Vec::new()),
        };

        PointerInputReport {
            target_node_id,
            bubble_node_ids,
            location: state.location,
            dispatched,
        }
    }
}
