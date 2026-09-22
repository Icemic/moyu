use crate::base::SurfaceSize;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PointerEventKind {
    Over,
    Enter,
    Leave,
    Down,
    Up,
    Click,
}

use super::Node;

pub trait Focusable: Node {
    /**
       Check whether the input point is on the node-like, coordinate of the point is relative to its parent.

       You may need to select property width and height, then handle `anchor` property to make it work.
       Or, if it is not square, you may need to do something more complex to calculate the correct area.
    */
    fn contains(&self, x: f32, y: f32, _: &FocusablePayload) -> bool {
        if self.base().content_bounds().contains(x, y) {
            return true;
        }

        false
    }

    /// Check whether descendants may participate in hit testing at the given local position.
    fn contains_children(&self, _: f32, _: f32, _: &FocusablePayload) -> bool {
        true
    }

    /// Receives a pointer event targeted at this node in local coordinates.
    fn pointer_event(&self, _: f32, _: f32, _: PointerEventKind) {}
}

#[derive(Debug)]
pub struct FocusablePayload {
    pub surface_size: SurfaceSize,
    pub stage_size: SurfaceSize,
}
