use std::ops::{Deref, DerefMut};

use bytemuck::Zeroable;
use glam::{Vec4, vec4};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable)]
pub struct Rect {
    vec4: Vec4,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            vec4: vec4(x, y, width, height),
        }
    }
    pub fn one() -> Self {
        Self { vec4: Vec4::ONE }
    }
    pub fn x(&self) -> f32 {
        self.vec4.x
    }
    pub fn y(&self) -> f32 {
        self.vec4.y
    }
    pub fn width(&self) -> f32 {
        self.vec4.z
    }
    pub fn height(&self) -> f32 {
        self.vec4.w
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::new(0., 0., 0., 0.)
    }
}

impl Deref for Rect {
    type Target = Vec4;

    fn deref(&self) -> &Self::Target {
        &self.vec4
    }
}

impl DerefMut for Rect {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec4
    }
}
