use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn one() -> Self {
        Self { x: 1.0, y: 1.0 }
    }
}

impl Default for Point {
    fn default() -> Self {
        Self::new(0., 0.)
    }
}
