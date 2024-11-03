use std::ops::{Deref, DerefMut};

use glam::Mat4;

/// | a | c | tx|
/// | b | d | ty|
/// | 0 | 0 | 1 |
///
/// tx, ty is pixel size
#[repr(C)]
#[derive(PartialEq, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Transform(Mat4);

impl Transform {
    /// create Transform instance
    pub fn new() -> Self {
        Self(Mat4::IDENTITY)
    }

    /// multiply with a transform
    pub fn multiply(&mut self, transform: Self) {
        self.0 *= transform.0;
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Transform {
    type Target = Mat4;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Transform {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
