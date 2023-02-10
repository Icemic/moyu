use std::sync::Arc;
use wgpu::{util::StagingBelt, BindGroup, BindGroupLayout, Buffer, CommandEncoder, Device, Queue};
use winit::dpi::LogicalSize;

use super::Node;

pub trait Renderable
where
    Self: Node,
{
    fn update(
        &mut self,
        arc_device: &Arc<Device>,
        arc_queue: &Arc<Queue>,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
        bind_group_layout: &BindGroupLayout,
        payload: &RendererUpdatePayload,
    );
    // pass arc because device are not used in every call.
    /// returns (bind_group, vertex_buffer)
    fn get_renderable(&self) -> Option<(&BindGroup, &Buffer)>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct RendererUpdatePayload {
    pub logical_size: LogicalSize<f64>,
    pub scale_factor: f64,
}
