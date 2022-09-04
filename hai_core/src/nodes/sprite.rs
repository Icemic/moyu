use hai_macros::node;
use log::warn;
use std::any::Any;
use std::sync::{Arc, Mutex, RwLock};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, Queue};
use winit::dpi::LogicalSize;

use crate::traits::{Node, NodeType, Renderable, RendererUpdatePayload, NODE_ID};
use crate::types::{Point, PointF, Transform};
use crate::{traits::Focusable, types::Vertex};

use super::{Texture, TextureStatus};

pub const SPRITE_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[node(renderable)]
#[derive(Debug)]
pub struct Sprite {
    /// loaded texture
    pub texture: Arc<RwLock<Texture>>,
    /// calculated vertices
    pub vertices: Option<[Vertex; 4]>,

    pub bind_group: Option<BindGroup>,
    pub vertex_buffer: Option<Buffer>,
}

impl Sprite {
    pub fn new(label: String, texture: Arc<RwLock<Texture>>) -> Self {
        let id = unsafe {
            NODE_ID += 1;
            NODE_ID
        };

        Sprite {
            id,
            label,
            anchor: PointF::default(),
            translate: Point::default(),
            transform: Transform::default(),
            transform_to_global: Transform::default(),
            children: vec![],

            texture,
            vertices: None,
            bind_group: None,
            vertex_buffer: None,
        }
    }

    fn calculate_vertices(&mut self, logical_size: LogicalSize<f64>, scale_factor: f64) {
        // (image_logical_size * image_scale_factor) / (screen_logical_size * screen_scale_factor) * coordinate_factor
        // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed
        let texture = self.texture.read().unwrap();
        let width =
            (texture.width as f64 * scale_factor) / (logical_size.width * scale_factor) * 2.;
        let height = (texture.height as f64 * scale_factor)
            / (logical_size.height * scale_factor) as f64
            * 2.;

        drop(texture);

        let a = self.transform_to_global.a;
        let b = self.transform_to_global.b;
        let c = self.transform_to_global.c;
        let d = self.transform_to_global.d;
        let tx = self.transform_to_global.tx;
        let ty = 1. - self.transform_to_global.ty;

        let w1 = -self.anchor.x * width;
        let w0 = w1 + width;
        let h1 = (-1. + self.anchor.y) * height;
        let h0 = h1 + height;

        // left top
        let p0x = a * w1 + c * h1 + tx - 1.;
        let p0y = b * w1 + d * h1 + ty;

        // left bottom
        let p1x = a * w0 + c * h1 + tx - 1.;
        let p1y = b * w0 + d * h1 + ty;

        // right top
        let p2x = a * w0 + c * h0 + tx - 1.;
        let p2y = b * w0 + d * h0 + ty;

        // right bottom
        let p3x = a * w1 + c * h0 + tx - 1.;
        let p3y = b * w1 + d * h0 + ty;

        let v = [
            Vertex {
                position: [p0x as f32, p0y as f32, 0.0],
                tex_coords: [0., 1.],
            },
            Vertex {
                position: [p1x as f32, p1y as f32, 0.0],
                tex_coords: [1., 1.],
            },
            Vertex {
                position: [p2x as f32, p2y as f32, 0.0],
                tex_coords: [1., 0.],
            },
            Vertex {
                position: [p3x as f32, p3y as f32, 0.0],
                tex_coords: [0., 0.],
            },
        ];

        self.vertices = Some(v);
    }
}

impl NodeType for Sprite {
    fn node_type(&self) -> &'static str {
        "sprite"
    }
}

impl Renderable for Sprite {
    fn update(
        &mut self,
        arc_device: &Arc<Mutex<Device>>,
        arc_queue: &Arc<Mutex<Queue>>,
        bind_group_layout: &BindGroupLayout,
        payload: &RendererUpdatePayload,
    ) {
        self.calculate_vertices(payload.logical_size, payload.scale_factor);

        let vertices = self.vertices.as_ref().unwrap();
        let device = arc_device.lock().unwrap();

        /*
         * bind group and vertex buffer should be created at the same time.
         * if bind_group (as well as vertex_buffer) is none, try to create it.
         */
        if self.bind_group.is_none() {
            let texture = self.texture.read().unwrap();
            if let TextureStatus::Ready = texture.status() {
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                texture.view.as_ref().unwrap(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(
                                texture.sampler.as_ref().unwrap(),
                            ),
                        },
                    ],
                    label: Some("bind_group"),
                });

                // release texture lock for better performance
                drop(texture);

                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });

                self.bind_group = Some(bind_group);
                self.vertex_buffer = Some(vertex_buffer);
            };
        } else {
            let queue = arc_queue.lock().unwrap();
            queue.write_buffer(
                self.vertex_buffer.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(self.vertices.as_ref().unwrap()),
            );
        }
    }

    fn get_renderable(&self) -> Option<(&BindGroup, &wgpu::Buffer)> {
        if self.bind_group.is_some() {
            Some((
                self.bind_group.as_ref().unwrap(),
                self.vertex_buffer.as_ref().unwrap(),
            ))
        } else {
            None
        }
    }
}

impl Focusable for Sprite {
    fn contains(&self, x: i32, y: i32) -> bool {
        let texture = self.texture.read().unwrap();

        if x > self.translate.x
            && x < texture.width as i32 + self.translate.x
            && y > self.translate.y
            && y < texture.height as i32 + self.translate.y
        {
            return true;
        }
        false
    }
}
