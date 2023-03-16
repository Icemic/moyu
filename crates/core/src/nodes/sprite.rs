use hai_macros::node;
use hai_pal::sync::RwLock;
use log::warn;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::Arc;
use wgpu::util::{DeviceExt, StagingBelt};
use wgpu::{BindGroup, BindGroupLayout, Buffer, CommandEncoder, Device, Queue};

use crate::core::get_core;
use crate::traits::{
    Focusable, Node, NodeType, Renderable, RendererUpdatePayload, UpdateProps, NODE_ID,
};
use crate::types::{Point, SurfaceSize, Transform, Vertex};
use crate::utils::calculate::calculate_rect_vertices;
use crate::utils::convert::{from_js, JSValue};

use super::{get_empty_texture, Texture, TextureStatus};

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

    fn calculate_vertices(&mut self, surface_size: &SurfaceSize) {
        // (image_logical_size * image_scale_factor) / (screen_logical_size * screen_scale_factor) * coordinate_factor
        // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed
        let (logical_width, logical_height) = surface_size.logical_size();
        let scale_factor = surface_size.scale_factor();
        let texture = self.texture.read();
        let width = (texture.width as f64 * scale_factor) / (logical_width * scale_factor) * 2.;
        let height =
            (texture.height as f64 * scale_factor) / (logical_height * scale_factor) as f64 * 2.;

        drop(texture);

        let vertices = calculate_rect_vertices(self, width, height, &self.area);

        self.vertices = Some(vertices);
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
        device: &Arc<Device>,
        _: &Arc<Queue>,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
        bind_group_layout: &BindGroupLayout,
        payload: &RendererUpdatePayload,
    ) {
        self.calculate_vertices(&payload.surface_size);

        let vertices = self.vertices.as_ref().unwrap();

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
        let props: SpriteProps = from_js(props).unwrap();

        if let Some(src) = props.src {
            let core = get_core();
            let mut resource_manager = core.resource_manager.lock();
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
