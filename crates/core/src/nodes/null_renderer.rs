use wgpu::util::StagingBelt;
use wgpu::*;

use crate::traits::{Node, RenderCommandSender, Renderer, RendererUpdatePayload};

/// A renderer that does nothing.
///
/// This is used when your node don't need to render anything.
pub struct VoidRenderer {}

impl VoidRenderer {
    pub fn new(_: &Device, _: &SurfaceConfiguration) -> Self {
        Self {}
    }
}

impl Renderer for VoidRenderer {
    fn name(&self) -> &'static str {
        "void"
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        unreachable!();
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        unreachable!();
    }

    fn update(
        &mut self,
        _: &mut dyn Node,
        _: &Device,
        _: &Queue,
        _: &mut CommandEncoder,
        _: &mut StagingBelt,
        _: &RendererUpdatePayload,
    ) {
        // do nothing
    }

    fn begin(&self) {}
    fn finish(&self) {}

    fn collect_commands(&self, _: &dyn Node, _: &RenderCommandSender) {}
}
