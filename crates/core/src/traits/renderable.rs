use std::sync::Arc;

use moyu_resource::ResourceManager;
use wgpu::{BindGroup, BindGroupLayout, Buffer, CommandEncoder, Device, Queue, util::StagingBelt};

use super::Node;

#[deprecated]
pub trait Renderable
where
    Self: Node,
{
    fn update(
        &mut self,
        arc_device: &Device,
        arc_queue: &Queue,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
        bind_group_layout: &BindGroupLayout,
        payload: &RendererUpdatePayload,
    );
    // pass arc because device are not used in every call.
    /// returns (bind_group, vertex_buffer)
    fn get_renderable(&self) -> Option<(&BindGroup, &Buffer)>;
}

#[derive(Debug)]
pub struct RendererUpdatePayload {
    /// time since app start, in seconds
    pub timestamp: f64,
    pub resource_manager: Arc<ResourceManager>,
}
