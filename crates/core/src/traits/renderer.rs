use wgpu::util::StagingBelt;
use wgpu::{BindGroupLayout, CommandEncoder, Device, Queue, RenderPipeline};

use crate::core::render_command::RenderQueue;

use super::{Node, RendererUpdatePayload};

pub trait Renderer {
    fn name(&self) -> &'static str;
    fn render_pipeline(&self) -> &RenderPipeline;
    fn bind_group_layout(&self) -> &BindGroupLayout;

    fn begin(&self) {}
    fn finish(&self) {}

    fn update(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
        payload: &RendererUpdatePayload,
    );

    fn collect_commands(&self, node: &dyn Node, render_queue: &mut RenderQueue);
    #[allow(unused_variables)]
    fn collect_post_commands(&self, node: &dyn Node, render_queue: &mut RenderQueue) {}
}
