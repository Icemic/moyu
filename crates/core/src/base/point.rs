use std::ops::{Deref, DerefMut};

use bytemuck::Zeroable;
use glam::{vec3a, Vec3A};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable)]
pub struct Point {
    vec3a: Vec3A,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            vec3a: vec3a(x, y, 1.0),
        }
    }
    pub fn one() -> Self {
        Self { vec3a: Vec3A::ONE }
    }
}

impl Default for Point {
    fn default() -> Self {
        Self::new(0., 0.)
    }
}

impl Deref for Point {
    type Target = Vec3A;

    fn deref(&self) -> &Self::Target {
        &self.vec3a
    }
}

impl DerefMut for Point {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec3a
    }
}
