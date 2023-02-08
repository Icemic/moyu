use hai_macros::node;
use hai_pal::sync::{Mutex, RwLock};
use log::warn;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::Arc;
use wgpu::util::{DeviceExt, StagingBelt};
use wgpu::{BindGroup, BindGroupLayout, Buffer, CommandEncoder, Device, Queue};
use winit::dpi::LogicalSize;

use crate::state::get_shared_state;
use crate::traits::{
    parse_props, JSValue, Node, NodeType, Renderable, RendererUpdatePayload, UpdateProps, NODE_ID,
};
use crate::types::{Point, Transform};
use crate::{traits::Focusable, types::Vertex};

use super::{get_empty_texture, Texture, TextureStatus};

pub const SPRITE_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[node(renderable)]
#[derive(Debug)]
pub struct Sprite {
    /// loaded texture
    pub texture: Arc<RwLock<Texture>>,
    /// clip area
    pub area: [f64; 4],
    /// calculated vertices
    pub vertices: Option<[Vertex; 4]>,

    pub bind_group: Option<BindGroup>,
    pub vertex_buffer: Option<Buffer>,
}

impl Sprite {
    pub fn new(label: String) -> Self {
        let id = unsafe {
            NODE_ID += 1;
            NODE_ID
        };

        Sprite {
            id,
            label,
            anchor: Point::default(),
            pivot: Point::default(),
            translate: Point::default(),
            scale: Point::one(),
            rotation: 0.,
            skew: Point::default(),

            _update_id: 0,
            _current_update_id: 1,

            transform: Transform::default(),
            global_transform: Transform::default(),
            children: vec![],

            texture: get_empty_texture().clone(),
            area: [0., 0., 1., 1.],
            vertices: None,
            bind_group: None,
            vertex_buffer: None,
        }
    }

    fn calculate_vertices(&mut self, logical_size: LogicalSize<f64>, scale_factor: f64) {
        // (image_logical_size * image_scale_factor) / (screen_logical_size * screen_scale_factor) * coordinate_factor
        // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed
        let texture = self.texture.read();
        let width =
            (texture.width as f64 * scale_factor) / (logical_size.width * scale_factor) * 2.;
        let height = (texture.height as f64 * scale_factor)
            / (logical_size.height * scale_factor) as f64
            * 2.;

        drop(texture);

        let [x0, y0, x1, y1] = self.area;

        // scale size to fit area
        let width = width * (x1 - x0);
        let height = height * (y1 - y0);

        let a = self.global_transform.a;
        let b = self.global_transform.b;
        let c = self.global_transform.c;
        let d = self.global_transform.d;
        let tx = self.global_transform.tx;
        let ty = 1. - self.global_transform.ty;

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
                tex_coords: [x0 as f32, y1 as f32],
            },
            Vertex {
                position: [p1x as f32, p1y as f32, 0.0],
                tex_coords: [x1 as f32, y1 as f32],
            },
            Vertex {
                position: [p2x as f32, p2y as f32, 0.0],
                tex_coords: [x1 as f32, y0 as f32],
            },
            Vertex {
                position: [p3x as f32, p3y as f32, 0.0],
                tex_coords: [x0 as f32, y0 as f32],
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
        _: &Arc<Mutex<Queue>>,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
        bind_group_layout: &BindGroupLayout,
        payload: &RendererUpdatePayload,
    ) {
        self.calculate_vertices(payload.logical_size, payload.scale_factor);

        let vertices = self.vertices.as_ref().unwrap();
        let device = arc_device.lock();

        /*
         * bind group and vertex buffer should be created at the same time.
         * if bind_group (as well as vertex_buffer) is none, try to create it.
         */
        if self.bind_group.is_none() {
            let texture = self.texture.read();
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
            let buf = bytemuck::cast_slice(self.vertices.as_ref().unwrap());
            staging_belt
                .write_buffer(
                    encoder,
                    self.vertex_buffer.as_ref().unwrap(),
                    0,
                    (buf.len() as u64).try_into().unwrap(),
                    &device,
                )
                .copy_from_slice(buf);
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
    fn contains(&self, x: f64, y: f64) -> bool {
        let texture = self.texture.read();

        let translate = self.translate();

        if x > translate.x
            && x < texture.width as f64 + translate.x
            && y > translate.y
            && y < texture.height as f64 + translate.y
        {
            return true;
        }
        false
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteProps {
    pub src: Option<String>,
    pub area: Option<[f64; 4]>,
}

impl UpdateProps for Sprite {
    fn update_properties(&mut self, props: &mut JSValue) {
        let props: SpriteProps = parse_props(props).unwrap();

        if let Some(src) = props.src {
            let state = get_shared_state();
            let state = state.read();
            let mut resource_manager = state.resource_manager.lock();
            let texture = resource_manager.get_texture(src);
            self.texture = texture;

            // drop old bind group
            self.bind_group = None;
        }

        if let Some(area) = props.area {
            self.area = area;
        }

        // force update vertices
        self._update_id += 1;
    }
}
