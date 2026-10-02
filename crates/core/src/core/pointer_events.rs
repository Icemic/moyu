use log::error;
use winit::dpi::PhysicalPosition;
use winit::event::{
    ElementState, MouseButton, MouseScrollDelta, TouchPhase as WinitTouchPhase, WindowEvent,
};
use winit::window::{CursorIcon, Window};

use crate::base::*;
use crate::core::FocusablePayload;
use crate::events::{
    MouseEvent, MouseEventKind, TouchEvent, TouchEventKind, WheelEvent, WheelEventDeltaMode,
    WheelEventKind,
};
use crate::state::{DeviceType, MOUSE_IDENTIFIER, PointerLocation, PointerState};
use crate::traits::PointerEventKind;
use crate::utils::dispatch_event::dispatch_event;
use crate::utils::hit_test::{get_local_logical_position, hit_test};

use super::Core;
use super::input::{PointerButton, TouchPhase};

macro_rules! get_pointer_state {
    ($self:ident, $name:ident, $identifier:expr) => {
        let pointer_state = $self.pointer_map.get(&$identifier);

        if pointer_state.is_none() {
            error!("Pointer state not found for identifier {}", $identifier);
            return;
        }

        let $name = pointer_state.unwrap();
    };

    ($self:ident, $name:ident, $identifier:expr, $ret:expr) => {
        let pointer_state = $self.pointer_map.get(&$identifier);

        if pointer_state.is_none() {
            error!("Pointer state not found for identifier {}", $identifier);
            return $ret;
        }

        let $name = pointer_state.unwrap();
    };
}

macro_rules! get_pointer_state_mut {
    ($self:ident, $name:ident, $identifier:expr) => {
        let pointer_state = $self.pointer_map.get_mut(&$identifier);

        if pointer_state.is_none() {
            error!("Pointer state not found for identifier {}", $identifier);
            return;
        }

        let mut $name = pointer_state.unwrap();
        let $name = $name.value_mut();
    };

    ($self:ident, $name:ident, $identifier:expr, $ret:expr) => {
        let pointer_state = $self.pointer_map.get_mut(&$identifier);

        if pointer_state.is_none() {
            error!("Pointer state not found for identifier {}", $identifier);
            return $ret;
        }

        let mut $name = pointer_state.unwrap();
        let $name = $name.value_mut();
    };
}

impl Core {
    pub fn handle_pointer_events(&self, window: &Window, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_pointer_move(window, position, MOUSE_IDENTIFIER);
                self.handle_pointer_hover(MOUSE_IDENTIFIER, true);

                true
            }
            // clear hover node when cursor leaves window
            WindowEvent::CursorLeft { .. } => {
                get_pointer_state!(self, pointer_state, MOUSE_IDENTIFIER, true);

                if let Some(last_hover_node) = &pointer_state.current_target {
                    let target_id = *last_hover_node.node.read().base().id();
                    dispatch_event(MouseEvent {
                        kind: MouseEventKind::MouseLeave,
                        target_id,
                        bubble_target_ids: last_hover_node.parent_ids.clone(),
                        location: pointer_state.location,
                    });
                    dispatch_pointer_event(
                        last_hover_node,
                        pointer_state.location,
                        PointerEventKind::Leave,
                    );
                }
                true
            }
            WindowEvent::MouseInput { button, state, .. } => {
                match state {
                    ElementState::Pressed => self.pointer_press(MOUSE_IDENTIFIER),
                    ElementState::Released => {
                        self.pointer_release(MOUSE_IDENTIFIER, (*button).into())
                    }
                }
                true
            }
            WindowEvent::Touch(touch) => {
                let identifier = touch.id as i32;

                self.get_ensure_pointer_state(identifier, DeviceType::Finger(identifier as u32));

                let last_location = {
                    get_pointer_state!(self, pointer_state, identifier, true);
                    pointer_state.location
                };

                let phase = match touch.phase {
                    WinitTouchPhase::Started => TouchPhase::Start,
                    WinitTouchPhase::Moved => TouchPhase::Move,
                    WinitTouchPhase::Ended => TouchPhase::End,
                    WinitTouchPhase::Cancelled => TouchPhase::Cancel,
                };

                self.handle_pointer_move(window, &touch.location, identifier);
                self.handle_pointer_hover(identifier, phase == TouchPhase::Start);

                // ignore duplicated touch move event
                let unchanged = self
                    .pointer_map
                    .get(&identifier)
                    .is_some_and(|pointer_state| pointer_state.location == last_location);

                if !(phase == TouchPhase::Move && unchanged) {
                    self.pointer_touch(identifier, phase);
                }

                true
            }
            WindowEvent::MouseWheel {
                delta, phase: _, ..
            } => {
                let (delta_x, delta_y, mode) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        (*x as f64, *y as f64, WheelEventDeltaMode::Line)
                    }
                    MouseScrollDelta::PixelDelta(position) => {
                        (position.x, position.y, WheelEventDeltaMode::Pixel)
                    }
                };

                self.pointer_wheel(MOUSE_IDENTIFIER, delta_x, delta_y, mode);

                true
            }
            _ => false,
        }
    }

    /// Handle hover changes on mouse move or touch, and record locations relative to client and screen (always in logical).
    fn handle_pointer_move(
        &self,
        window: &Window,
        position: &PhysicalPosition<f64>,
        identifier: i32,
    ) {
        get_pointer_state_mut!(self, pointer_state, identifier);

        let stage_size = {
            let stage_size = self.stage_size.read();
            *stage_size
        };

        let (scale, translate_x, translate_y) = {
            let stage_transform = self.stage_transform.read();
            *stage_transform
        };

        let window_position = window.inner_position().unwrap_or_default();
        let scale_factor = stage_size.scale_factor();

        let global_logical_x = (position.x / scale_factor) as f32;
        let global_logical_y = (position.y / scale_factor) as f32;

        let screen_logical_x = ((window_position.x as f64 + position.x) / scale_factor) as f32;
        let screen_logical_y = ((window_position.y as f64 + position.y) / scale_factor) as f32;

        let stage_logical_x = (global_logical_x - translate_x) / scale;
        let stage_logical_y = (global_logical_y - translate_y) / scale;

        pointer_state.location.valid = true;
        pointer_state.location.client_x = stage_logical_x.round() as i32;
        pointer_state.location.client_y = stage_logical_y.round() as i32;
        pointer_state.location.screen_x = screen_logical_x.round() as i32;
        pointer_state.location.screen_y = screen_logical_y.round() as i32;
    }

    pub(super) fn handle_pointer_hover(&self, identifier: i32, refresh_hover_node: bool) {
        let surface_size = {
            let surface_size = self.surface_size.read();
            *surface_size
        };

        let stage_size = {
            let stage_size = self.stage_size.read();
            *stage_size
        };

        let upload_payload = FocusablePayload {
            surface_size,
            stage_size,
        };

        get_pointer_state_mut!(self, pointer_state, identifier);

        let last_hover_node = &mut pointer_state.current_target;

        if refresh_hover_node && pointer_state.location.valid {
            // get node under pointer
            if let Some(node) = hit_test(
                &self.root_node(),
                pointer_state.location.client_x as f32,
                pointer_state.location.client_y as f32,
                &upload_payload,
            ) {
                let node_ref = node.node.read();
                let (x, y) = get_local_logical_position(
                    node_ref.as_ref(),
                    pointer_state.location.client_x as f32,
                    pointer_state.location.client_y as f32,
                );

                pointer_state.location.offset_x = x;
                pointer_state.location.offset_y = y;

                drop(node_ref);

                if identifier == MOUSE_IDENTIFIER {
                    let target_id = *node.node.read().base().id();

                    dispatch_event(MouseEvent {
                        kind: MouseEventKind::MouseMove,
                        target_id,
                        bubble_target_ids: node.parent_ids.clone(),
                        location: pointer_state.location,
                    });
                    dispatch_pointer_event(&node, pointer_state.location, PointerEventKind::Over);

                    if let Some(last_hover_node) = last_hover_node {
                        if last_hover_node == &node {
                            // do nothing if last focused node is the same as current node
                            return;
                        }

                        let node_ref = last_hover_node.node.read();
                        let (x, y) = get_local_logical_position(
                            node_ref.as_ref(),
                            pointer_state.location.client_x as f32,
                            pointer_state.location.client_y as f32,
                        );

                        let mut location = pointer_state.location;

                        location.offset_x = x;
                        location.offset_y = y;

                        let target_id = *node_ref.base().id();

                        // drop node guard before dispatching event, since it may cause deadlock
                        drop(node_ref);

                        // if last focused node is different from current node, it's a mouse leave event and a mouse enter event
                        dispatch_event(MouseEvent {
                            kind: MouseEventKind::MouseLeave,
                            target_id,
                            bubble_target_ids: last_hover_node.parent_ids.clone(),
                            location,
                        });
                        dispatch_pointer_event(last_hover_node, location, PointerEventKind::Leave);
                    }

                    // there is always a mouse enter event if current node is different from last focused node (may be None)
                    dispatch_event(MouseEvent {
                        kind: MouseEventKind::MouseEnter,
                        target_id,
                        bubble_target_ids: node.parent_ids.clone(),
                        location: pointer_state.location,
                    });
                    dispatch_pointer_event(&node, pointer_state.location, PointerEventKind::Enter);
                }

                self.set_cursor(node.node.read().base().cursor().clone());

                // record last focused node
                *last_hover_node = Some(node);

                return;
            } else {
                *last_hover_node = None;
            }
        }

        // if no node under pointer, it's a mouse leave event if last focused node is not None
        if let Some(last_hover_node) = last_hover_node {
            let node_ref = last_hover_node.node.read();
            let (x, y) = get_local_logical_position(
                node_ref.as_ref(),
                pointer_state.location.client_x as f32,
                pointer_state.location.client_y as f32,
            );

            pointer_state.location.offset_x = x;
            pointer_state.location.offset_y = y;

            if identifier == MOUSE_IDENTIFIER {
                let target_id = *node_ref.base().id();

                // drop node guard before dispatching event, since it may cause deadlock
                drop(node_ref);

                dispatch_event(MouseEvent {
                    kind: MouseEventKind::MouseLeave,
                    target_id,
                    bubble_target_ids: last_hover_node.parent_ids.clone(),
                    location: pointer_state.location,
                });
                dispatch_pointer_event(
                    last_hover_node,
                    pointer_state.location,
                    PointerEventKind::Leave,
                );

                self.set_cursor(MoyuCursor::Visible(CursorIcon::Default));
            }
        }
    }

    /// Press at the pointer's current target.
    fn pointer_press(&self, identifier: i32) {
        get_pointer_state_mut!(self, pointer_state, identifier);

        let Some(last_hover_node) = &pointer_state.current_target else {
            return;
        };

        let target_id = *last_hover_node.node.read().base().id();
        let bubble_target_ids = last_hover_node.parent_ids.clone();
        let location = pointer_state.location;

        self.editable.handle_pointer_down(target_id);

        dispatch_event(MouseEvent {
            kind: MouseEventKind::MouseDown,
            target_id,
            bubble_target_ids,
            location,
        });
        dispatch_pointer_event(last_hover_node, location, PointerEventKind::Down);

        pointer_state.down_id = Some(target_id);
    }

    /// Release at the pointer's current target, adding the gesture the button carries.
    fn pointer_release(&self, identifier: i32, button: PointerButton) {
        get_pointer_state_mut!(self, pointer_state, identifier);

        let Some(last_hover_node) = &pointer_state.current_target else {
            return;
        };

        let target_id = *last_hover_node.node.read().base().id();
        let bubble_target_ids = last_hover_node.parent_ids.clone();
        let location = pointer_state.location;

        dispatch_event(MouseEvent {
            kind: MouseEventKind::MouseUp,
            target_id,
            bubble_target_ids: bubble_target_ids.clone(),
            location,
        });
        dispatch_pointer_event(last_hover_node, location, PointerEventKind::Up);

        // A release only counts as a gesture when it pairs with a press on the same node.
        let down_id = pointer_state.down_id.take();

        if down_id != Some(target_id) {
            return;
        }

        match button {
            PointerButton::Left => {
                dispatch_event(MouseEvent {
                    kind: MouseEventKind::Click,
                    target_id,
                    bubble_target_ids,
                    location,
                });
                dispatch_pointer_event(last_hover_node, location, PointerEventKind::Click);
            }
            PointerButton::Right => {
                dispatch_event(MouseEvent {
                    kind: MouseEventKind::ContextMenu,
                    target_id,
                    bubble_target_ids,
                    location,
                });
            }
            PointerButton::Middle
            | PointerButton::Back
            | PointerButton::Forward
            | PointerButton::Other => {
                // no gesture is attached to these buttons
            }
        }
    }

    /// Dispatch one touch phase to the pointer's current target.
    fn pointer_touch(&self, identifier: i32, phase: TouchPhase) {
        get_pointer_state_mut!(self, pointer_state, identifier);

        let Some(last_hover_node) = &pointer_state.current_target else {
            return;
        };

        let target_id = *last_hover_node.node.read().base().id();
        let bubble_target_ids = last_hover_node.parent_ids.clone();
        let location = pointer_state.location;
        let touch_identifier = identifier as u32;

        match phase {
            TouchPhase::Start => {
                self.editable.handle_pointer_down(target_id);

                dispatch_event(TouchEvent {
                    kind: TouchEventKind::TouchStart,
                    target_id,
                    bubble_target_ids,
                    location,
                    identifier: touch_identifier,
                });
                dispatch_pointer_event(last_hover_node, location, PointerEventKind::Down);

                pointer_state.down_id = Some(target_id);
            }
            TouchPhase::Move => {
                dispatch_event(TouchEvent {
                    kind: TouchEventKind::TouchMove,
                    target_id,
                    bubble_target_ids,
                    location,
                    identifier: touch_identifier,
                });
                dispatch_pointer_event(last_hover_node, location, PointerEventKind::Over);
            }
            TouchPhase::End => {
                dispatch_event(TouchEvent {
                    kind: TouchEventKind::TouchEnd,
                    target_id,
                    bubble_target_ids,
                    location,
                    identifier: touch_identifier,
                });
                dispatch_pointer_event(last_hover_node, location, PointerEventKind::Up);

                pointer_state.down_id.take();
            }
            TouchPhase::Cancel => {
                dispatch_event(TouchEvent {
                    kind: TouchEventKind::TouchCancel,
                    target_id,
                    bubble_target_ids,
                    location,
                    identifier: touch_identifier,
                });
                dispatch_pointer_event(last_hover_node, location, PointerEventKind::Leave);

                pointer_state.down_id = None;
            }
        }
    }

    /// Scroll the pointer's current target.
    fn pointer_wheel(
        &self,
        identifier: i32,
        delta_x: f64,
        delta_y: f64,
        mode: WheelEventDeltaMode,
    ) {
        get_pointer_state!(self, pointer_state, identifier);

        let Some(last_hover_node) = &pointer_state.current_target else {
            return;
        };

        dispatch_event(WheelEvent {
            kind: WheelEventKind::Wheel,
            target_id: *last_hover_node.node.read().base().id(),
            bubble_target_ids: last_hover_node.parent_ids.clone(),
            delta_x,
            delta_y,
            delta_z: 0.0,
            delta_mode: mode,
        });
    }

    /// Check if the pointer state exists, if not, create one.
    fn get_ensure_pointer_state(&self, identifier: i32, device_type: DeviceType) {
        self.pointer_map.entry(identifier).or_insert_with(|| {
            let mut pointer_state = PointerState::default();
            pointer_state.device_type = device_type;
            pointer_state
        });
    }
}

impl From<MouseButton> for PointerButton {
    fn from(button: MouseButton) -> Self {
        match button {
            MouseButton::Left => Self::Left,
            MouseButton::Right => Self::Right,
            MouseButton::Middle => Self::Middle,
            MouseButton::Back => Self::Back,
            MouseButton::Forward => Self::Forward,
            MouseButton::Other(_) => Self::Other,
        }
    }
}

fn dispatch_pointer_event(
    target: &crate::utils::hit_test::HitTestTarget,
    location: PointerLocation,
    kind: PointerEventKind,
) {
    let node = target.node.read();
    if let Some(focusable) = node.as_focusable() {
        focusable.pointer_event(location.offset_x, location.offset_y, kind);
    }
}
