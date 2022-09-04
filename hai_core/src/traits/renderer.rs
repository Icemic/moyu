use wgpu::{BindGroupLayout, Buffer, RenderPipeline};

pub trait Renderer
where
    Self: Send,
{
    fn name(&self) -> &'static str;
    fn render_pipeline(&self) -> &RenderPipeline;
    fn bind_group_layout(&self) -> &BindGroupLayout;
    fn index_buffer(&self) -> &Buffer;
}
