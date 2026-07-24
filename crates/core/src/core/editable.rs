use std::sync::Arc;

use moyu_pal::sync::{Mutex, RwLock};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{Ime, KeyEvent};
use winit::keyboard::ModifiersState;
use winit::window::Window;

use super::{NodeLock, NodeMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImeState {
    Disabled,
    PendingEnable { target_id: u32 },
    Enabled { target_id: u32 },
}

struct EditableManagerState {
    active_target: Option<u32>,
    ime: ImeState,
}

pub struct EditableManager {
    window: Arc<Window>,
    node_map: NodeMap,
    stage_transform: Arc<RwLock<(f32, f32, f32)>>,
    state: Mutex<EditableManagerState>,
}

impl EditableManager {
    pub(super) fn new(
        window: Arc<Window>,
        node_map: NodeMap,
        stage_transform: Arc<RwLock<(f32, f32, f32)>>,
    ) -> Self {
        Self {
            window,
            node_map,
            stage_transform,
            state: Mutex::new(EditableManagerState {
                active_target: None,
                ime: ImeState::Disabled,
            }),
        }
    }

    pub fn focus(&self, node_id: u32) -> bool {
        let Some(node) = self.resolve_focus_target(node_id) else {
            return false;
        };

        if self.active_target() == Some(node_id) {
            return true;
        }

        self.blur_active();

        node.write().as_editable_target_mut().unwrap().did_focus();
        self.state.lock().active_target = Some(node_id);
        true
    }

    pub fn blur(&self, node_id: u32) -> bool {
        if self.active_target() != Some(node_id) {
            return false;
        }
        self.blur_active()
    }

    /// Clears the active focus before notifying the node so callbacks observe the
    /// final manager state. A detached node is still considered successfully cleared.
    pub(super) fn blur_active(&self) -> bool {
        let Some(node_id) = self.state.lock().active_target.take() else {
            return false;
        };
        let Some(node) = self.node_map.get(&node_id).map(|node| node.clone()) else {
            return true;
        };
        let mut node = node.write();
        if let Some(editable) = node.as_editable_target_mut() {
            editable.cancel_composition();
            editable.did_blur();
        }
        true
    }

    pub(super) fn handle_pointer_down(&self, target_id: u32) {
        if !self.focus(target_id) {
            self.blur_active();
        }
    }

    pub(super) fn handle_keyboard_input(&self, event: &KeyEvent, modifiers: ModifiersState) {
        self.revalidate_active_target();

        let Some(node_id) = self.active_target() else {
            return;
        };
        let Some(node) = self.node_map.get(&node_id).map(|node| node.clone()) else {
            return;
        };
        let mut node = node.write();
        if let Some(editable) = node.as_editable_target_mut() {
            editable.handle_keyboard_input(event, modifiers);
        }
    }

    /// Routes composition data only to the editable owned by the enabled IME state.
    ///
    /// `Ime::Enabled` is a platform lifecycle notification, not an acknowledgement
    /// for `set_ime_allowed`, so it is used only to refresh the candidate position.
    pub(super) fn handle_ime(&self, event: &Ime) {
        let Some(target_id) = self.enabled_target() else {
            return;
        };

        if matches!(event, Ime::Enabled) {
            self.update_ime_cursor_area(target_id);
        }
        if !matches!(event, Ime::Preedit(_, _) | Ime::Commit(_)) {
            return;
        }
        let Some(node) = self.node_map.get(&target_id).map(|node| node.clone()) else {
            return;
        };
        let mut node = node.write();
        if let Some(editable) = node.as_editable_target_mut() {
            editable.handle_ime(event);
        }
    }

    pub(super) fn maintain(&self) {
        self.revalidate_active_target();
        self.reconcile_ime();
        self.settle_pending_clear();
        self.update_active_ime_cursor_area();
    }

    fn active_target(&self) -> Option<u32> {
        self.state.lock().active_target
    }

    fn enabled_target(&self) -> Option<u32> {
        let state = self.state.lock();
        match state.ime {
            ImeState::Enabled { target_id } if state.active_target == Some(target_id) => {
                Some(target_id)
            }
            _ => None,
        }
    }

    fn revalidate_active_target(&self) {
        let Some(node_id) = self.active_target() else {
            return;
        };
        if self.resolve_focus_target(node_id).is_none() {
            self.blur_active();
        }
    }

    /// Resolves an eligible editable that remains reachable through visible nodes.
    fn resolve_focus_target(&self, target_id: u32) -> Option<NodeLock> {
        let root = self.node_map.get(&0).unwrap().clone();
        let node = find_visible_node(&root, target_id)?;
        {
            let node = node.read();
            let editable = node.as_editable_target()?;
            if editable.is_disabled() {
                return None;
            }
        }

        Some(node)
    }

    /// Reconciles the platform IME lifecycle with the currently eligible editable.
    ///
    /// Switching targets disables the current IME first and leaves the next target
    /// in `PendingEnable`. A later event-loop turn enables it, which prevents a
    /// disable/enable pair from being collapsed into the same platform callback.
    fn reconcile_ime(&self) {
        let desired_target = self.desired_ime_target();
        let ime_allowed = {
            let mut state = self.state.lock();
            match state.ime {
                ImeState::Disabled => desired_target.map(|target_id| {
                    state.ime = ImeState::Enabled { target_id };
                    true
                }),
                ImeState::PendingEnable { target_id } => match desired_target {
                    None => {
                        state.ime = ImeState::Disabled;
                        None
                    }
                    Some(desired_id) if desired_id != target_id => {
                        state.ime = ImeState::PendingEnable {
                            target_id: desired_id,
                        };
                        None
                    }
                    Some(_) => {
                        state.ime = ImeState::Enabled { target_id };
                        Some(true)
                    }
                },
                ImeState::Enabled { target_id } if desired_target != Some(target_id) => {
                    state.ime = match desired_target {
                        Some(target_id) => ImeState::PendingEnable { target_id },
                        None => ImeState::Disabled,
                    };
                    Some(false)
                }
                ImeState::Enabled { .. } => None,
            }
        };

        if let Some(allowed) = ime_allowed {
            self.window.set_ime_allowed(allowed);
        }
    }

    fn settle_pending_clear(&self) {
        let Some(target_id) = self.enabled_target() else {
            return;
        };
        let Some(node) = self.node_map.get(&target_id).map(|node| node.clone()) else {
            return;
        };
        let mut node = node.write();
        if let Some(editable) = node.as_editable_target_mut() {
            editable.settle_pending_clear();
        }
    }

    fn update_active_ime_cursor_area(&self) {
        let Some(target_id) = self.enabled_target() else {
            return;
        };
        self.update_ime_cursor_area(target_id);
    }

    fn desired_ime_target(&self) -> Option<u32> {
        if !self.window.has_focus() {
            return None;
        }
        let target_id = self.active_target()?;
        let node = self.node_map.get(&target_id)?;
        let node = node.read();
        let editable = node.as_editable_target()?;
        (!editable.is_disabled() && !editable.is_read_only()).then_some(target_id)
    }

    /// Converts the editable-local caret rectangle into physical window pixels for
    /// the platform candidate window, accounting for node, stage, and DPI transforms.
    fn update_ime_cursor_area(&self, target_id: u32) {
        if self.active_target() != Some(target_id) {
            return;
        }
        let Some(node) = self.node_map.get(&target_id).map(|node| node.clone()) else {
            return;
        };
        let stage_rect = {
            let node = node.read();
            let Some(editable) = node.as_editable_target() else {
                return;
            };
            let Some(rect) = editable.ime_cursor_rect() else {
                return;
            };
            rect.into_bound().transform(node.base().global_transform())
        };

        let (scale, translate_x, translate_y) = *self.stage_transform.read();
        let scale_factor = self.window.scale_factor() as f32;
        let x = ((stage_rect.min_x() * scale + translate_x) * scale_factor).round() as i32;
        let y = ((stage_rect.min_y() * scale + translate_y) * scale_factor).round() as i32;
        let width = (stage_rect.width() * scale * scale_factor).max(0.0).round() as u32;
        let height = (stage_rect.height() * scale * scale_factor)
            .max(0.0)
            .round() as u32;

        self.window.set_ime_cursor_area(
            PhysicalPosition::new(x, y),
            PhysicalSize::new(width, height),
        );
    }
}

fn find_visible_node(node: &NodeLock, target_id: u32) -> Option<NodeLock> {
    let (node_id, visible, children) = {
        let node = node.read();
        (
            *node.base().id(),
            node.base().visible(),
            node.base().children().clone(),
        )
    };
    if !visible {
        return None;
    }
    if node_id == target_id {
        return Some(node.clone());
    }

    for child in children {
        if let Some(node) = find_visible_node(&child, target_id) {
            return Some(node);
        }
    }
    None
}
