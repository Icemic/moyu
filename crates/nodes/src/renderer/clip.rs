use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::{Node, RenderCommandSender, Renderer, RendererUpdatePayload};
use moyu_core::utils::coordinates::calculate_layout_rect;
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
        _render_queue: &RenderCommandSender,
        payload: &RendererUpdatePayload,
    ) {
        // Calculate clip rect.
        // Cannot use node.bounds() since we need to limit to our width/height.
        let rect = calculate_layout_rect(
            node,
            payload.stage_logical_size.0,
            payload.stage_logical_size.1,
        );

        let clip = node.as_any_mut().downcast_mut::<Clip>().unwrap();
        clip.rect = rect;
    }

    fn collect_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        let clip = node.as_any().downcast_ref::<Clip>().unwrap();

        render_queue
            .send(RenderCommand::BeginClip { rect: clip.rect })
            .unwrap();
    }

    fn collect_post_commands(&self, _node: &dyn Node, render_queue: &RenderCommandSender) {
        render_queue.send(RenderCommand::EndClip).unwrap();
    }
}
