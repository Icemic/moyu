use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl Default for Point {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

#[repr(C)]
#[derive(PartialEq, Copy, Clone, Debug, Pod, Zeroable)]
pub struct PointF {
    pub x: f64,
    pub y: f64,
}

impl PointF {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl Default for PointF {
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
    pub fn translate(tx: f64, ty: f64) -> Self {
        Self {
            tx,
            ty,
            ..Default::default()
        }
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
