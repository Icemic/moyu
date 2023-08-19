use super::{Node, RendererUpdatePayload};

pub trait Focusable: Node {
    /// check whether the input point is on the node-like,
    /// coordinate of the point is relative to its parent
    fn contains(&self, x: f32, y: f32, payload: &RendererUpdatePayload) -> bool;
}
