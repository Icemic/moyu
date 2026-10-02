use std::ops::{Deref, DerefMut};

use glam::{Affine3A, vec3a};

/// | a | c | tx|
/// | b | d | ty|
/// | 0 | 0 | 1 |
///
/// tx, ty is pixel size
#[repr(C)]
#[derive(PartialEq, Copy, Clone, Debug, bytemuck::Zeroable)]
pub struct Transform(Affine3A);

impl Transform {
    /// create Transform instance
    pub fn new() -> Self {
        Self(Affine3A::IDENTITY)
    }

    /// multiply with a transform
    pub fn multiply(&mut self, transform: Self) {
        self.0 *= transform.0;
    }

    /// Apply this transform to a point, treating its z as 0.
    pub fn transform_point(&self, x: f32, y: f32) -> (f32, f32) {
        let point = self.0.transform_point3a(vec3a(x, y, 0.0));

        (point.x, point.y)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Transform {
    type Target = Affine3A;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Transform {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
