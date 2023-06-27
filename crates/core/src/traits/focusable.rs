use super::RendererUpdatePayload;

pub trait Focusable {
    /// check whether the input point is on the node-like,
    /// coordinate of the point is relative to its parent
    fn contains(&self, x: f64, y: f64, payload: &RendererUpdatePayload) -> bool;
}
