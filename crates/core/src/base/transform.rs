use std::ops::{Deref, DerefMut};

use bytemuck::Zeroable;
use glam::Affine2;

/// | a | c | tx|
/// | b | d | ty|
/// | 0 | 0 | 1 |
///
/// tx, ty is pixel size
#[repr(C)]
#[derive(PartialEq, Copy, Clone, Debug, Zeroable)]
pub struct Transform {
    affine: Affine2,
}

impl Transform {
    /// create Transform instance
    pub fn new() -> Self {
        let affine = Affine2::default();
        Transform { affine }
    }

    /// multiply with a transform
    pub fn multiply(&mut self, transform: Self) {
        self.affine = self.affine * transform.affine;
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Transform {
    type Target = Affine2;

    fn deref(&self) -> &Self::Target {
        &self.affine
    }
}

impl DerefMut for Transform {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.affine
    }
}
