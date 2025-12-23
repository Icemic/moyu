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
