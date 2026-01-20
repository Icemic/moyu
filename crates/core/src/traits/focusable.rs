use crate::base::SurfaceSize;

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
}

#[derive(Debug)]
pub struct FocusablePayload {
    pub surface_size: SurfaceSize,
    pub stage_size: SurfaceSize,
}
