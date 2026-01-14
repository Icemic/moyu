use std::ops::{Deref, DerefMut};

use bytemuck::Zeroable;
use glam::{Vec4, vec3a, vec4};

use crate::base::Rect;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable)]
pub struct Bound {
    vec4: Vec4,
}

impl Bound {
    pub fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            vec4: vec4(min_x, min_y, max_x, max_y),
        }
    }

    pub fn min_x(&self) -> f32 {
        self.vec4.x
    }
    pub fn min_y(&self) -> f32 {
        self.vec4.y
    }
    pub fn max_x(&self) -> f32 {
        self.vec4.z
    }
    pub fn max_y(&self) -> f32 {
        self.vec4.w
    }

    pub fn width(&self) -> f32 {
        self.max_x() - self.min_x()
    }

    pub fn height(&self) -> f32 {
        self.max_y() - self.min_y()
    }

    pub fn is_empty(&self) -> bool {
        self.width() <= 0.0 || self.height() <= 0.0
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.min_x() && x <= self.max_x() && y >= self.min_y() && y <= self.max_y()
    }

    pub fn union(&self, other: &Self) -> Self {
        let min_x = self.min_x().min(other.min_x());
        let min_y = self.min_y().min(other.min_y());
        let max_x = self.max_x().max(other.max_x());
        let max_y = self.max_y().max(other.max_y());

        Self::new(min_x, min_y, max_x, max_y)
    }

    pub fn clamp(&self, min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        let clamped_min_x = self.min_x().max(min_x);
        let clamped_min_y = self.min_y().max(min_y);
        let clamped_max_x = self.max_x().min(max_x);
        let clamped_max_y = self.max_y().min(max_y);

        Self::new(clamped_min_x, clamped_min_y, clamped_max_x, clamped_max_y)
    }

    pub fn transform(&self, transform: &super::transform::Transform) -> Self {
        if self.is_empty() {
            return *self;
        }

        let p1 = transform.transform_point3a(vec3a(self.min_x(), self.min_y(), 0.0));
        let p2 = transform.transform_point3a(vec3a(self.max_x(), self.min_y(), 0.0));
        let p3 = transform.transform_point3a(vec3a(self.min_x(), self.max_y(), 0.0));
        let p4 = transform.transform_point3a(vec3a(self.max_x(), self.max_y(), 0.0));

        let min_x = p1.x.min(p2.x).min(p3.x).min(p4.x);
        let min_y = p1.y.min(p2.y).min(p3.y).min(p4.y);
        let max_x = p1.x.max(p2.x).max(p3.x).max(p4.x);
        let max_y = p1.y.max(p2.y).max(p3.y).max(p4.y);

        Self::new(min_x, min_y, max_x, max_y)
    }

    pub fn into_rect(&self) -> Rect {
        self.into()
    }
}

impl Default for Bound {
    fn default() -> Self {
        Self::new(0., 0., 0., 0.)
    }
}

impl Deref for Bound {
    type Target = Vec4;

    fn deref(&self) -> &Self::Target {
        &self.vec4
    }
}

impl DerefMut for Bound {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec4
    }
}

impl From<Rect> for Bound {
    fn from(rect: Rect) -> Self {
        Self::new(
            rect.x(),
            rect.y(),
            rect.x() + rect.width(),
            rect.y() + rect.height(),
        )
    }
}

impl From<&Rect> for Bound {
    fn from(rect: &Rect) -> Self {
        Self::new(
            rect.x(),
            rect.y(),
            rect.x() + rect.width(),
            rect.y() + rect.height(),
        )
    }
}
