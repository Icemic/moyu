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
    pub tx: i32,
    pub ty: i32,
}

impl Transform {
    pub fn new(a: f64, b: f64, c: f64, d: f64, tx: i32, ty: i32) -> Self {
        Transform { a, b, c, d, tx, ty }
    }

    pub fn translate(tx: i32, ty: i32) -> Self {
        Self {
            tx,
            ty,
            ..Default::default()
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new(1., 0., 0., 1., 0, 0)
    }
}
