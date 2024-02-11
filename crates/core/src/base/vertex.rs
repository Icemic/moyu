use bytemuck::{Pod, Zeroable};

/// trait for defining a vertex type using in wgpu
pub trait VertexDesc: Sized {
    /// get vertex attributes
    fn attribs() -> &'static [wgpu::VertexAttribute];
    /// Get vertex buffer layout.
    /// In general you don't need to override this method
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::attribs(),
        }
    }
}

/// Built-in vertex type for sprite rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub tint: [f32; 4],
}

impl VertexDesc for SpriteVertex {
    fn attribs() -> &'static [wgpu::VertexAttribute] {
        static SPRITE_ATTRIBS: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];

        &SPRITE_ATTRIBS
    }
}
