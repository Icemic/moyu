use moyu_core::core::render_command::{RenderCommand, RenderQueue};
use moyu_core::traits::{Node, Renderer, RendererUpdatePayload};
use moyu_core::utils::coordinates::calculate_bounding_box;
use wgpu::util::StagingBelt;
use wgpu::*;

use crate::nodes::Clip;

pub struct ClipRenderer;

impl ClipRenderer {
    pub fn new(_device: &Device, _config: &SurfaceConfiguration) -> Self {
        Self {}
    }
}

impl Renderer for ClipRenderer {
    fn name(&self) -> &'static str {
        "clip"
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        unreachable!()
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        unreachable!()
    }

    fn update(
        &mut self,
        node: &mut dyn Node,
        _device: &Device,
        _queue: &Queue,
        _encoder: &mut CommandEncoder,
        _staging_belt: &mut StagingBelt,
        payload: &RendererUpdatePayload,
    ) {
        let rect = calculate_bounding_box(
            node,
            payload.stage_logical_size.0,
            payload.stage_logical_size.1,
        );

        let clip = node.as_any_mut().downcast_mut::<Clip>().unwrap();
        clip.rect = rect;
    }

    fn collect_commands(&self, node: &dyn Node, render_queue: &mut RenderQueue) {
        let clip = node.as_any().downcast_ref::<Clip>().unwrap();

        if let Some(rect) = &clip.rect {
            render_queue.push(RenderCommand::BeginClip { rect: *rect });
        }
    }

    fn collect_post_commands(&self, _node: &dyn Node, render_queue: &mut RenderQueue) {
        render_queue.push(RenderCommand::EndClip);
    }
}
