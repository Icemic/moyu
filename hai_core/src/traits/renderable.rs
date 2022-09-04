use std::sync::{Arc, Mutex};

use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, Queue};
use winit::dpi::LogicalSize;

use crate::types::Transform;

use super::Node;

pub trait Renderable
where
    Self: Node,
{
    fn update(
        &mut self,
        arc_device: &Arc<Mutex<Device>>,
        arc_queue: &Arc<Mutex<Queue>>,
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
