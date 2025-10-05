use std::ops::{Deref, DerefMut};

use glam::Affine3A;

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
