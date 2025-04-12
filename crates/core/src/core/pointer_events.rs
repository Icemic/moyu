use log::error;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, TouchPhase, WindowEvent};
use winit::window::{CursorIcon, Window};

use crate::base::*;
use crate::core::FocusablePayload;
use crate::events::{MouseEvent, MouseEventKind, TouchEvent, TouchEventKind};
use crate::state::{DeviceType, PointerLocation, PointerState, MOUSE_IDENTIFIER};
use crate::utils::dispatch_event::dispatch_event;
use crate::utils::hit_test::{get_local_logical_position, hit_test};

use super::Core;

macro_rules! get_pointer_state {
    ($self:ident, $name:ident, $identifier:expr) => {
        let pointer_map = $self.pointer_map.read();
        let pointer_state = pointer_map.get(&$identifier);

        if pointer_state.is_none() {
            error!("Pointer state not found for identifier {}", $identifier);
            return;
        }

        let $name = pointer_state.unwrap();
    };

    ($self:ident, $name:ident, $identifier:expr, $ret:expr) => {
        let pointer_map = $self.pointer_map.read();
        let pointer_state = pointer_map.get(&$identifier);

        if pointer_state.is_none() {
            error!("Pointer state not found for identifier {}", $identifier);
            return $ret;
        }

        let $name = pointer_state.unwrap();
    };
}

macro_rules! get_pointer_state_mut {
    ($self:ident, $name:ident, $identifier:expr) => {
        let mut pointer_map = $self.pointer_map.write();
        let pointer_state = pointer_map.get_mut(&$identifier);

        if pointer_state.is_none() {
            error!("Pointer state not found for identifier {}", $identifier);
            return;
        }

        let $name = pointer_state.unwrap();
    };

    ($self:ident, $name:ident, $identifier:expr, $ret:expr) => {
        let mut pointer_map = $self.pointer_map.write();
        let pointer_state = pointer_map.get_mut(&$identifier);

        if pointer_state.is_none() {
            error!("Pointer state not found for identifier {}", $identifier);
            return $ret;
        }

        let $name = pointer_state.unwrap();
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
                }
                true
            }
            WindowEvent::MouseInput { button, state, .. } => {
                get_pointer_state_mut!(self, pointer_state, MOUSE_IDENTIFIER, true);

                if let Some(last_hover_node) = &pointer_state.current_target {
                    let target_id = *last_hover_node.node.read().base().id();
                    let bubble_target_ids = last_hover_node.parent_ids.clone();

                    let location = pointer_state.location;

                    match state {
                        ElementState::Pressed => {
                            dispatch_event(MouseEvent {
                                kind: MouseEventKind::MouseDown,
                                target_id,
                                bubble_target_ids,
                                location,
                            });
                            if let winit::event::MouseButton::Left = button {
                                pointer_state.down_id = Some(target_id);
                            }
                        }
                        ElementState::Released => {
                            dispatch_event(MouseEvent {
                                kind: MouseEventKind::MouseUp,
                                target_id,
                                bubble_target_ids: bubble_target_ids.clone(),
                                location,
                            });

                            let down_id = pointer_state.down_id.take();

                            if down_id == Some(target_id) {
                                match button {
                                    winit::event::MouseButton::Left => {
                                        dispatch_event(MouseEvent {
                                            kind: MouseEventKind::Click,
                                            target_id,
                                            bubble_target_ids,
                                            location,
                                        });
                                    }
                                    winit::event::MouseButton::Right => {
                                        dispatch_event(MouseEvent {
                                            kind: MouseEventKind::ContextMenu,
                                            target_id,
                                            bubble_target_ids,
                                            location,
                                        });
                                    }
                                    winit::event::MouseButton::Back => {
                                        // do nothing
                                    }
                                    winit::event::MouseButton::Forward => {
                                        // do nothing
                                    }
                                    winit::event::MouseButton::Middle => {
                                        // do nothing
                                    }
                                    winit::event::MouseButton::Other(_) => {
                                        // do nothing
                                    }
                                }
                            }
                        }
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

                self.handle_pointer_move(window, &touch.location, identifier);
                self.handle_pointer_hover(identifier, touch.phase == TouchPhase::Started);

                get_pointer_state!(self, pointer_state, identifier, true);

                if last_location == pointer_state.location && touch.phase == TouchPhase::Moved {
                    // ignore duplicated touch move event
                    return true;
                }

                if let Some(last_hover_node) = &pointer_state.current_target {
                    let target_id = *last_hover_node.node.read().base().id();
                    let bubble_target_ids = last_hover_node.parent_ids.clone();

                    let location = pointer_state.location;

                    match touch.phase {
                        TouchPhase::Started => {
                            dispatch_event(TouchEvent {
                                kind: TouchEventKind::TouchStart,
                                target_id,
                                bubble_target_ids,
                                location,
                                identifier: Some(touch.id as u32),
                            });
                        }
                        TouchPhase::Moved => {
                            dispatch_event(TouchEvent {
                                kind: TouchEventKind::TouchMove,
                                target_id,
                                bubble_target_ids,
                                location,
                                identifier: Some(touch.id as u32),
                            });
                        }
                        TouchPhase::Ended => {
                            dispatch_event(TouchEvent {
                                kind: TouchEventKind::TouchEnd,
                                target_id,
                                bubble_target_ids,
                                location,
                                identifier: Some(touch.id as u32),
                            });
                        }
                        TouchPhase::Cancelled => {
                            dispatch_event(TouchEvent {
                                kind: TouchEventKind::TouchCancel,
                                target_id,
                                bubble_target_ids,
                                location,
                                identifier: Some(touch.id as u32),
                            });
                        }
                    }
                }
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

        let location = PointerLocation {
            client_x: stage_logical_x.round() as u32,
            client_y: stage_logical_y.round() as u32,
            screen_x: screen_logical_x.round() as u32,
            screen_y: screen_logical_y.round() as u32,
            layer_x: 0.,
            layer_y: 0.,
        };

        pointer_state.location = location;
    }

    fn handle_pointer_hover(&self, identifier: i32, refresh_hover_node: bool) {
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

        if refresh_hover_node {
            // get node under pointer
            if let Some(node) = hit_test(
                &self.root_node,
                pointer_state.location.client_x as f32,
                pointer_state.location.client_y as f32,
                &upload_payload,
            ) {
                let node_ref = node.node.read();
                let (x, y) = get_local_logical_position(
                    &*node_ref,
                    pointer_state.location.client_x as f32,
                    pointer_state.location.client_y as f32,
                );

                pointer_state.location.layer_x = x;
                pointer_state.location.layer_y = y;

                drop(node_ref);

                if identifier == MOUSE_IDENTIFIER {
                    let target_id = *node.node.read().base().id();

                    dispatch_event(MouseEvent {
                        kind: MouseEventKind::MouseMove,
                        target_id,
                        bubble_target_ids: node.parent_ids.clone(),
                        location: pointer_state.location,
                    });

                    if let Some(last_hover_node) = last_hover_node {
                        if last_hover_node == &node {
                            // do nothing if last focused node is the same as current node
                            return;
                        }

                        let node_ref = last_hover_node.node.read();
                        let (x, y) = get_local_logical_position(
                            &*node_ref,
                            pointer_state.location.client_x as f32,
                            pointer_state.location.client_y as f32,
                        );

                        let mut location = pointer_state.location;

                        location.layer_x = x;
                        location.layer_y = y;

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
                    }

                    // there is always a mouse enter event if current node is different from last focused node (may be None)
                    dispatch_event(MouseEvent {
                        kind: MouseEventKind::MouseEnter,
                        target_id,
                        bubble_target_ids: node.parent_ids.clone(),
                        location: pointer_state.location,
                    });
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
                &*node_ref,
                pointer_state.location.client_x as f32,
                pointer_state.location.client_y as f32,
            );

            pointer_state.location.layer_x = x;
            pointer_state.location.layer_y = y;

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

                self.set_cursor(MoyuCursor::Visible(CursorIcon::Default));
            }
        }
    }

    /// Check if the pointer state exists, if not, create one.
    fn get_ensure_pointer_state(&self, identifier: i32, device_type: DeviceType) {
        let mut pointer_map = self.pointer_map.write();
        pointer_map.entry(identifier).or_insert_with(|| {
            let mut pointer_state = PointerState::default();
            pointer_state.device_type = device_type;
            pointer_state
        });
    }
}
