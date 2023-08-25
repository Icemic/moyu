use super::{Node, RendererUpdatePayload};

pub trait Focusable: Node {
    /**
       Check whether the input point is on the node-like, coordinate of the point is relative to its parent.

       You may need to select property width and height, then handle `anchor` property to make it work.
       Or, if it is not square, you may need to do something more complex to calculate the correct area.
    */
    fn contains(&self, x: f32, y: f32, payload: &RendererUpdatePayload) -> bool;
}
