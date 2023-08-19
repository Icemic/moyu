use std::sync::Arc;

use wgpu::util::StagingBelt;
use wgpu::{BindGroupLayout, CommandEncoder, Device, Queue, RenderPass, RenderPipeline};

use super::{Node, RendererUpdatePayload};

pub trait Renderer
where
    Self: Send + Sync,
{
    fn name(&self) -> &'static str;
    fn render_pipeline(&self) -> &RenderPipeline;
    fn bind_group_layout(&self) -> &BindGroupLayout;

    fn begin(&self) {}
    fn finish(&self) {}

    fn update(
        &mut self,
        node: &mut dyn Node,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
        payload: &RendererUpdatePayload,
    );
    fn render<'a, 'b: 'a>(
        &'b self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        render_pass: &mut RenderPass<'a>,
        node: &'b dyn Node,
    );
}
