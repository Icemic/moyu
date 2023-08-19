use bytemuck::{Pod, Zeroable};
use winit::dpi::PhysicalSize;

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

/// | a | c | tx|
/// | b | d | ty|
/// | 0 | 0 | 1 |
///
/// tx, ty is pixel size
#[repr(C)]
#[derive(PartialEq, Copy, Clone, Debug, Pod, Zeroable)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

impl Transform {
    /// create Transform instance
    pub fn new(a: f64, b: f64, c: f64, d: f64, tx: f64, ty: f64) -> Self {
        Transform { a, b, c, d, tx, ty }
    }

    /// create Transform instance from specific translate value
    #[allow(dead_code)]
    pub fn translate(tx: f64, ty: f64) -> Point {
        Point { x: tx, y: ty }
    }

    /// set translate value
    #[allow(dead_code)]
    pub fn set_translate(&mut self, x: f64, y: f64) {
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

#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct SurfaceSize {
    /// logical width
    width: f64,
    /// logical height
    height: f64,
    /// scale factor, aka _device pixel ratio_
    scale_factor: f64,
}

#[allow(dead_code)]
impl SurfaceSize {
    pub fn new(logical_width: f64, logical_height: f64, scale_factor: f64) -> Self {
        Self {
            width: logical_width,
            height: logical_height,
            scale_factor,
        }
    }

    pub fn from_physical_size(physical_size: &PhysicalSize<u32>, scale_factor: f64) -> Self {
        let width = physical_size.width as f64 / scale_factor;
        let height = physical_size.height as f64 / scale_factor;

        Self {
            width,
            height,
            scale_factor,
        }
    }

    pub fn logical_size(&self) -> (f64, f64) {
        (self.width, self.height)
    }

    pub fn physical_size(&self) -> (u32, u32) {
        let width = (self.width * self.scale_factor) as u32;
        let height = (self.height * self.scale_factor) as u32;
        (width, height)
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub fn set_logical_size(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
    }

    pub fn set_physical_size(&mut self, width: u32, height: u32) {
        self.width = width as f64 / self.scale_factor;
        self.height = height as f64 / self.scale_factor;
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
    }
}
