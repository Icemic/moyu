use crate::base::Rect;
use crate::core::{ImeEvent, KeyInput};

use super::Node;

pub trait EditableTarget: Node {
    fn is_disabled(&self) -> bool;
    fn is_read_only(&self) -> bool;
    fn did_focus(&mut self);
    fn did_blur(&mut self);
    fn handle_keyboard_input(&mut self, input: &KeyInput);
    fn handle_ime(&mut self, event: &ImeEvent);
    fn settle_pending_clear(&mut self);
    fn cancel_composition(&mut self);
    fn ime_cursor_rect(&self) -> Option<Rect>;
}
