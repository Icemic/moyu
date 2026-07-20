use std::sync::mpsc::SyncSender;

use crate::base::Bound;
use wgpu::{BindGroupLayout, Device, Queue, RenderPipeline};

use crate::core::render_command::RenderCommand;

use super::{Node, RendererUpdatePayload};

pub type RenderCommandSender = SyncSender<RenderCommand>;

pub trait Renderer {
    fn name(&self) -> &'static str;
    fn render_pipeline(&self) -> &RenderPipeline;
    fn bind_group_layout(&self) -> &BindGroupLayout;

    fn begin(&self) {}
    fn finish(&self) {}

    #[allow(unused_variables)]
    fn prepare(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        queue: &Queue,
        payload: &RendererUpdatePayload,
    ) {
    }

    fn update(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        queue: &Queue,
        render_queue: &RenderCommandSender,
        payload: &RendererUpdatePayload,
    );

    fn should_collect_commands(&self, node: &dyn Node, stage_bound: &Bound) -> bool {
        node.base().visible() && node.base().global_content_bounds().intersects(stage_bound)
    }

    fn collect_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender);
    #[allow(unused_variables)]
    fn collect_post_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {}
}
