use std::time::Duration;

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
use crate::state::{
    ClickRecord, DeviceType, MOUSE_IDENTIFIER, MousePress, PointerLocation, PointerState,
    TapGesture,
};
use crate::traits::PointerEventKind;
use crate::utils::dispatch_event::dispatch_event;
use crate::utils::hit_test::{get_local_logical_position, hit_test};

use super::Core;
use super::input::{DispatchRecord, MouseEventSource, PointerButton, TouchPhase};

/// Two left clicks on the same target within this interval, and with little movement
/// between them, are reported as a double click. The window is implementation defined
/// on the web; the engine uses a fixed value.
const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(500);

/// Maximum movement between two left clicks still counted as a double click, in stage
/// logical pixels.
const DOUBLE_CLICK_DISTANCE: f32 = 4.0;

/// Maximum distance a touch may travel and still be a tap, in stage logical pixels. A
/// tap produces the mouse compatibility gesture browsers send, so this plays the role
/// the browser's own tap slop does.
const TAP_SLOP: f32 = 12.0;

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
                self.handle_pointer_hover(
                    MOUSE_IDENTIFIER,
                    true,
                    MouseEventSource::User,
                    &mut DispatchRecord::disabled(),
                );

                true
            }
            // clear hover node when cursor leaves window
            WindowEvent::CursorLeft { .. } => {
                get_pointer_state!(self, pointer_state, MOUSE_IDENTIFIER, true);

                if let Some(last_hover_node) = &pointer_state.current_target {
                    let target_id = *last_hover_node.node.read().base().id();
                    dispatch_event(self.mouse_event(
                        MouseEventKind::MouseLeave,
                        target_id,
                        last_hover_node.parent_ids.clone(),
                        0,
                        buttons_mask(&pointer_state),
                        pointer_state.location,
                        MouseEventSource::User,
                    ));
                    dispatch_pointer_event(
                        last_hover_node,
                        pointer_state.location,
                        PointerEventKind::Leave,
                    );
                }
                true
            }
            WindowEvent::MouseInput { button, state, .. } => {
                // Only the buttons DOM defines produce events; other physical buttons
                // are ignored.
                if let Some(button) = from_winit_button(*button) {
                    match state {
                        ElementState::Pressed => self.pointer_press(
                            MOUSE_IDENTIFIER,
                            button,
                            MouseEventSource::User,
                            &mut DispatchRecord::disabled(),
                        ),
                        ElementState::Released => self.pointer_release(
                            MOUSE_IDENTIFIER,
                            button,
                            MouseEventSource::User,
                            &mut DispatchRecord::disabled(),
                        ),
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

                let mut record = DispatchRecord::disabled();

                self.handle_pointer_move(window, &touch.location, identifier);
                self.handle_pointer_hover(
                    identifier,
                    phase == TouchPhase::Start,
                    MouseEventSource::User,
                    &mut record,
                );

                // ignore duplicated touch move event
                let unchanged = self
                    .pointer_map
                    .get(&identifier)
                    .is_some_and(|pointer_state| pointer_state.location == last_location);

                if !(phase == TouchPhase::Move && unchanged) {
                    self.pointer_touch(identifier, phase, &mut record);
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

                self.pointer_wheel(
                    MOUSE_IDENTIFIER,
                    delta_x,
                    delta_y,
                    mode,
                    &mut DispatchRecord::disabled(),
                );

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

        self.store_pointer_location(
            identifier,
            stage_logical_x,
            stage_logical_y,
            screen_logical_x,
            screen_logical_y,
        );
    }

    /// Record a pointer's position.
    ///
    /// `stage_x`/`stage_y` are stage logical units; `screen_x`/`screen_y` additionally
    /// include the window's position on the desktop. Both are rounded, as the window
    /// event path always did.
    fn store_pointer_location(
        &self,
        identifier: i32,
        stage_x: f32,
        stage_y: f32,
        screen_x: f32,
        screen_y: f32,
    ) {
        get_pointer_state_mut!(self, pointer_state, identifier);

        pointer_state.location.valid = true;
        pointer_state.location.client_x = stage_x.round() as i32;
        pointer_state.location.client_y = stage_y.round() as i32;
        pointer_state.location.screen_x = screen_x.round() as i32;
        pointer_state.location.screen_y = screen_y.round() as i32;
    }

    /// Move a pointer to a position on the stage.
    ///
    /// Window events arrive in surface pixels and go through `handle_pointer_move`;
    /// this is the inverse, for callers that already work in stage coordinates.
    pub(super) fn pointer_to(&self, identifier: i32, x: f32, y: f32) {
        let stage_size = {
            let stage_size = self.stage_size.read();
            *stage_size
        };

        let (scale, translate_x, translate_y) = {
            let stage_transform = self.stage_transform.read();
            *stage_transform
        };

        let window_position = self.window.inner_position().unwrap_or_default();
        let scale_factor = stage_size.scale_factor();

        let screen_x = (window_position.x as f64 / scale_factor) as f32 + x * scale + translate_x;
        let screen_y = (window_position.y as f64 / scale_factor) as f32 + y * scale + translate_y;

        self.store_pointer_location(identifier, x, y, screen_x, screen_y);
    }

    pub(super) fn handle_pointer_hover(
        &self,
        identifier: i32,
        refresh_hover_node: bool,
        source: MouseEventSource,
        record: &mut DispatchRecord<'_>,
    ) {
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

        let buttons = buttons_mask(pointer_state);

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

                    dispatch_event(self.mouse_event(
                        MouseEventKind::MouseMove,
                        target_id,
                        node.parent_ids.clone(),
                        0,
                        buttons,
                        pointer_state.location,
                        source,
                    ));
                    dispatch_pointer_event(&node, pointer_state.location, PointerEventKind::Over);
                    record.push("MouseMove");

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
                        dispatch_event(self.mouse_event(
                            MouseEventKind::MouseLeave,
                            target_id,
                            last_hover_node.parent_ids.clone(),
                            0,
                            buttons,
                            location,
                            source,
                        ));
                        dispatch_pointer_event(last_hover_node, location, PointerEventKind::Leave);
                        record.push("MouseLeave");
                    }

                    // there is always a mouse enter event if current node is different from last focused node (may be None)
                    dispatch_event(self.mouse_event(
                        MouseEventKind::MouseEnter,
                        target_id,
                        node.parent_ids.clone(),
                        0,
                        buttons,
                        pointer_state.location,
                        source,
                    ));
                    dispatch_pointer_event(&node, pointer_state.location, PointerEventKind::Enter);
                    record.push("MouseEnter");
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

                dispatch_event(self.mouse_event(
                    MouseEventKind::MouseLeave,
                    target_id,
                    last_hover_node.parent_ids.clone(),
                    0,
                    buttons,
                    pointer_state.location,
                    source,
                ));
                dispatch_pointer_event(
                    last_hover_node,
                    pointer_state.location,
                    PointerEventKind::Leave,
                );
                record.push("MouseLeave");

                self.set_cursor(MoyuCursor::Visible(CursorIcon::Default));
            }
        }
    }

    /// Press a mouse button at the pointer's current target.
    pub(super) fn pointer_press(
        &self,
        identifier: i32,
        button: PointerButton,
        source: MouseEventSource,
        record: &mut DispatchRecord<'_>,
    ) {
        get_pointer_state_mut!(self, pointer_state, identifier);

        let Some(last_hover_node) = &pointer_state.current_target else {
            return;
        };

        let target_id = *last_hover_node.node.read().base().id();
        let bubble_target_ids = last_hover_node.parent_ids.clone();
        let location = pointer_state.location;

        self.editable.handle_pointer_down(target_id);

        // Record the press before dispatching, so the event's mask includes the button
        // being pressed.
        pointer_state.mouse_downs[button.index()] = Some(MousePress {
            node_id: target_id,
            parent_ids: bubble_target_ids.clone(),
        });
        let buttons = buttons_mask(pointer_state);

        dispatch_event(self.mouse_event(
            MouseEventKind::MouseDown,
            target_id,
            bubble_target_ids.clone(),
            button.as_button_number(),
            buttons,
            location,
            source,
        ));
        dispatch_pointer_event(last_hover_node, location, PointerEventKind::Down);
        record.push("MouseDown");

        // The engine follows Chromium on Linux: the right-button press opens the
        // context menu, independent of the release that follows.
        if button == PointerButton::Right {
            dispatch_event(self.mouse_event(
                MouseEventKind::ContextMenu,
                target_id,
                bubble_target_ids,
                button.as_button_number(),
                buttons,
                location,
                source,
            ));
            record.push("ContextMenu");
        }
    }

    /// Release a mouse button at the pointer's current target, dispatching the click
    /// gesture the release completes.
    pub(super) fn pointer_release(
        &self,
        identifier: i32,
        button: PointerButton,
        source: MouseEventSource,
        record: &mut DispatchRecord<'_>,
    ) {
        get_pointer_state_mut!(self, pointer_state, identifier);

        let Some(last_hover_node) = &pointer_state.current_target else {
            return;
        };

        let target_id = *last_hover_node.node.read().base().id();
        let bubble_target_ids = last_hover_node.parent_ids.clone();
        let location = pointer_state.location;

        // The button is no longer held, so it drops out of the mask the event carries.
        let press = pointer_state.mouse_downs[button.index()].take();
        let buttons = buttons_mask(pointer_state);

        dispatch_event(self.mouse_event(
            MouseEventKind::MouseUp,
            target_id,
            bubble_target_ids.clone(),
            button.as_button_number(),
            buttons,
            location,
            source,
        ));
        dispatch_pointer_event(last_hover_node, location, PointerEventKind::Up);
        record.push("MouseUp");

        // A release only completes a gesture when the same button was pressed first.
        let Some(press) = press else {
            return;
        };

        // The gesture targets the nearest common ancestor of the press and release
        // targets. Ancestor chains are ordered root first and exclude the root, so
        // their longest shared prefix ends at that ancestor; an empty shared prefix
        // means the root itself.
        let common_len = press
            .parent_ids
            .iter()
            .copied()
            .chain(std::iter::once(press.node_id))
            .zip(
                bubble_target_ids
                    .iter()
                    .copied()
                    .chain(std::iter::once(target_id)),
            )
            .take_while(|(a, b)| a == b)
            .count();

        let (gesture_target, gesture_bubble) = if common_len == 0 {
            (0, Vec::new())
        } else {
            (
                press
                    .parent_ids
                    .get(common_len - 1)
                    .copied()
                    .unwrap_or(press.node_id),
                press.parent_ids[..common_len - 1].to_vec(),
            )
        };

        match button {
            PointerButton::Left => {
                dispatch_event(self.mouse_event(
                    MouseEventKind::Click,
                    gesture_target,
                    gesture_bubble.clone(),
                    0,
                    buttons,
                    location,
                    source,
                ));
                record.push("Click");

                // Node-level clicks keep their existing meaning: the press and the
                // release landed on the same node.
                if press.node_id == target_id {
                    dispatch_pointer_event(last_hover_node, location, PointerEventKind::Click);
                }

                let now = self.instant.elapsed();
                let position = (location.client_x as f32, location.client_y as f32);
                let repeated = pointer_state.last_click.as_ref().is_some_and(|last| {
                    last.target_id == gesture_target
                        && now.saturating_sub(last.at) <= DOUBLE_CLICK_INTERVAL
                        && (last.x - position.0).powi(2) + (last.y - position.1).powi(2)
                            <= DOUBLE_CLICK_DISTANCE * DOUBLE_CLICK_DISTANCE
                });

                if repeated {
                    dispatch_event(self.mouse_event(
                        MouseEventKind::DoubleClick,
                        gesture_target,
                        gesture_bubble,
                        0,
                        buttons,
                        location,
                        source,
                    ));
                    record.push("DoubleClick");
                    pointer_state.last_click = None;
                } else {
                    pointer_state.last_click = Some(ClickRecord {
                        target_id: gesture_target,
                        x: position.0,
                        y: position.1,
                        at: now,
                    });
                }
            }
            PointerButton::Middle
            | PointerButton::Right
            | PointerButton::Back
            | PointerButton::Forward => {
                dispatch_event(self.mouse_event(
                    MouseEventKind::AuxClick,
                    gesture_target,
                    gesture_bubble,
                    button.as_button_number(),
                    buttons,
                    location,
                    source,
                ));
                record.push("AuxClick");
            }
        }
    }

    /// Dispatch one touch phase to the pointer's current target.
    ///
    /// A tap — a touch that starts and ends within [`TAP_SLOP`] while no other touch is
    /// down — additionally produces the mouse compatibility gesture browsers send: the
    /// mouse pointer moves to where the finger lifted and the ordinary press / release
    /// pair runs there, so hover state, cursor, click target and double click detection
    /// all follow the mouse path.
    pub(super) fn pointer_touch(
        &self,
        identifier: i32,
        phase: TouchPhase,
        record: &mut DispatchRecord<'_>,
    ) {
        // A second finger makes the gesture multi-touch, which produces no mouse
        // compatibility events. The touches already down are cancelled before this
        // pointer's state is locked: their state lives in the same map, and taking a
        // second shard while holding one deadlocks.
        let multi_touch = phase == TouchPhase::Start && self.cancel_other_touches(identifier);

        let mut tap_position = None;

        {
            get_pointer_state_mut!(self, pointer_state, identifier);

            let location = pointer_state.location;
            let touch_identifier = identifier as u32;

            // Give up on the tap once the finger travels past the slop. Every move is
            // checked: a finger that leaves and comes back may end near its start while
            // the gesture was a drag.
            if phase == TouchPhase::Move {
                if let Some(tap) = &mut pointer_state.tap {
                    let dx = location.client_x as f32 - tap.start_x;
                    let dy = location.client_y as f32 - tap.start_y;
                    if dx * dx + dy * dy > TAP_SLOP * TAP_SLOP {
                        tap.cancelled = true;
                    }
                }
            }

            let Some(last_hover_node) = &pointer_state.current_target else {
                return;
            };

            let target_id = *last_hover_node.node.read().base().id();
            let bubble_target_ids = last_hover_node.parent_ids.clone();

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
                    record.push("TouchStart");

                    pointer_state.touch_down_id = Some(target_id);
                    pointer_state.tap = Some(TapGesture {
                        start_x: location.client_x as f32,
                        start_y: location.client_y as f32,
                        cancelled: multi_touch,
                    });
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
                    record.push("TouchMove");
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
                    record.push("TouchEnd");

                    pointer_state.touch_down_id.take();

                    if pointer_state.tap.take().is_some_and(|tap| !tap.cancelled) {
                        tap_position = Some((location.client_x as f32, location.client_y as f32));
                    }
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
                    record.push("TouchCancel");

                    pointer_state.touch_down_id = None;
                    pointer_state.tap = None;
                }
            }
        }

        if let Some((x, y)) = tap_position {
            self.synthesize_tap_click(x, y, record);
        }
    }

    /// Cancel the tap candidate on every touch other than `identifier`, reporting
    /// whether any of them was down.
    ///
    /// Called when a new touch starts: from that point on the gesture is multi-touch,
    /// so neither the touches already down nor the new one produces a mouse
    /// compatibility click. Iterating the map without holding a shard is why the caller
    /// runs this before locking the starting touch's state.
    fn cancel_other_touches(&self, identifier: i32) -> bool {
        let mut found = false;

        for mut entry in self.pointer_map.iter_mut() {
            if *entry.key() == identifier {
                continue;
            }

            let state = entry.value_mut();
            if state.touch_down_id.is_some() {
                found = true;
            }
            if let Some(tap) = state.tap.as_mut() {
                tap.cancelled = true;
            }
        }

        found
    }

    /// Produce the mouse compatibility gesture of a tap: move the mouse pointer to
    /// where the finger lifted, then press and release the left button there.
    ///
    /// Both steps reuse the mouse path, so the synthesized click goes through the same
    /// hover update, hit testing, gesture target resolution and double click detection
    /// as a real one. The events carry `synthetic`, since the user operated a finger
    /// rather than a mouse.
    fn synthesize_tap_click(&self, x: f32, y: f32, record: &mut DispatchRecord<'_>) {
        self.pointer_to(MOUSE_IDENTIFIER, x, y);
        self.handle_pointer_hover(MOUSE_IDENTIFIER, true, MouseEventSource::Synthetic, record);
        self.pointer_press(
            MOUSE_IDENTIFIER,
            PointerButton::Left,
            MouseEventSource::Synthetic,
            record,
        );
        self.pointer_release(
            MOUSE_IDENTIFIER,
            PointerButton::Left,
            MouseEventSource::Synthetic,
            record,
        );
    }

    /// Scroll the pointer's current target.
    pub(super) fn pointer_wheel(
        &self,
        identifier: i32,
        delta_x: f64,
        delta_y: f64,
        mode: WheelEventDeltaMode,
        record: &mut DispatchRecord<'_>,
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
        record.push("Wheel");
    }

    /// Check if the pointer state exists, if not, create one.
    pub(super) fn get_ensure_pointer_state(&self, identifier: i32, device_type: DeviceType) {
        self.pointer_map.entry(identifier).or_insert_with(|| {
            let mut pointer_state = PointerState::default();
            pointer_state.device_type = device_type;
            pointer_state
        });
    }
}

/// Map a winit mouse button to the engine's vocabulary. DOM only defines the five
/// standard buttons, so any other physical button yields `None` and is ignored.
fn from_winit_button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Left),
        MouseButton::Middle => Some(PointerButton::Middle),
        MouseButton::Right => Some(PointerButton::Right),
        MouseButton::Back => Some(PointerButton::Back),
        MouseButton::Forward => Some(PointerButton::Forward),
        MouseButton::Other(_) => None,
    }
}

/// Bitmask of the mouse buttons currently held, in the DOM `MouseEvent.buttons` encoding.
fn buttons_mask(pointer_state: &PointerState) -> u32 {
    PointerButton::ALL
        .iter()
        .zip(&pointer_state.mouse_downs)
        .filter_map(|(button, press)| press.as_ref().map(|_| button.bit()))
        .sum()
}

impl Core {
    /// Build a mouse event carrying the button it is about, the buttons held and the
    /// modifier keys currently down.
    fn mouse_event(
        &self,
        kind: MouseEventKind,
        target_id: u32,
        bubble_target_ids: Vec<u32>,
        button: i32,
        buttons: u32,
        location: PointerLocation,
        source: MouseEventSource,
    ) -> MouseEvent {
        let modifiers = self.modifiers_state.load();

        MouseEvent {
            kind,
            target_id,
            bubble_target_ids,
            button,
            buttons,
            ctrl_key: modifiers.control_key(),
            shift_key: modifiers.shift_key(),
            alt_key: modifiers.alt_key(),
            meta_key: modifiers.super_key(),
            synthetic: source == MouseEventSource::Synthetic,
            location,
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
