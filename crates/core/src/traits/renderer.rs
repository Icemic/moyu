use std::sync::Arc;

use wgpu::util::StagingBelt;
use wgpu::{BindGroupLayout, CommandEncoder, Device, Queue, RenderPass, RenderPipeline};

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
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
        payload: &RendererUpdatePayload,
    );
    fn render(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        render_pass: &mut RenderPass,
        node: &dyn Node,
    );
}
