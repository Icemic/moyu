use bytemuck::{Pod, Zeroable};

use super::Point;

/// | a | c | tx|
/// | b | d | ty|
/// | 0 | 0 | 1 |
///
/// tx, ty is pixel size
#[repr(C)]
#[derive(PartialEq, Copy, Clone, Debug, Pod, Zeroable)]
pub struct Transform {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub tx: f32,
    pub ty: f32,
}

impl Transform {
    /// create Transform instance
    pub fn new(a: f32, b: f32, c: f32, d: f32, tx: f32, ty: f32) -> Self {
        Transform { a, b, c, d, tx, ty }
    }

    /// create Transform instance from specific translate value
    #[allow(dead_code)]
    pub fn translate(tx: f32, ty: f32) -> Point {
        Point::new(tx, ty)
    }

    /// set translate value
    #[allow(dead_code)]
    pub fn set_translate(&mut self, x: f32, y: f32) {
        self.tx = x;
        self.ty = y;
    }

    /// multiply with a transform
    pub fn multiply(&mut self, transform: Self) {
        let a = self.a;
        let b = self.b;
        let c = self.c;
        let d = self.d;

        self.a = (transform.a * a) + (transform.b * c);
        self.b = (transform.a * b) + (transform.b * d);
        self.c = (transform.c * a) + (transform.d * c);
        self.d = (transform.c * b) + (transform.d * d);

        self.tx = (transform.tx * a) + (transform.ty * c) + self.tx;
        self.ty = (transform.tx * b) + (transform.ty * d) + self.ty;
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new(1., 0., 0., 1., 0., 0.)
    }
}
