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
