use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::StagingBelt;
use wgpu::{util::DeviceExt, *};

use crate::base::*;
#[cfg(feature = "video")]
use crate::nodes::Video;
use crate::nodes::{Sprite, Texture, TextureStatus};
use crate::resource::TextureId;
use crate::traits::{Node, NodeBaseTrait, RendererUpdatePayload};
use crate::utils::calculate::calculate_rect_vertices;
use crate::{traits::Renderer, utils::constants::RECTANGLE_INDICES};

/// the number of vertices in a sprite is always 4.
// pub static NUM_VERTICES: u32 = 4;

static NUM_INDICES: u32 = RECTANGLE_INDICES.len() as u32;

pub struct SpriteRenderer {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    index_buffer: Buffer,
    bind_group_map: HashMap<Arc<TextureId>, BindGroup>,
}

impl SpriteRenderer {
    pub fn new(device: &Arc<Device>, config: &SurfaceConfiguration) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(
                        // SamplerBindingType::Comparison is only for TextureSampleType::Depth
                        // SamplerBindingType::Filtering if the sample_type of the texture is:
                        //     TextureSampleType::Float { filterable: true }
                        // Otherwise you'll get an error.
                        SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        // shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Sprite Shader"),
            source: ShaderSource::Wgsl(include_str!("./shaders/default.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Sprite Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Sprite Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[SpriteVertex::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // index buffers for each sprite are always the same.
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Renderer Index Buffer"),
            contents: bytemuck::cast_slice(RECTANGLE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            pipeline,
            bind_group_layout,
            index_buffer,
            bind_group_map: Default::default(),
        }
    }
}

impl SpriteRenderer {
    fn get_bind_group(&mut self, device: &Device, texture: &Arc<Texture>) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        texture.view.load().as_ref().unwrap(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(
                        texture.sampler.load().as_ref().unwrap(),
                    ),
                },
            ],
            label: Some("bind_group"),
        })
    }
}

impl Renderer for SpriteRenderer {
    fn name(&self) -> &'static str {
        return "sprite";
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    fn update(
        &mut self,
        node: &mut dyn Node,
        device: &Arc<Device>,
        _: &Arc<Queue>,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
        payload: &RendererUpdatePayload,
    ) {
        // (image_logical_size * image_scale_factor) / (screen_logical_size * screen_scale_factor) * coordinate_factor
        // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed
        let (logical_width, logical_height) = payload.surface_size.logical_size();
        let scale_factor = payload.surface_size.scale_factor();

        let node = node.as_any_mut().downcast_mut::<Sprite>().unwrap();

        if let Some(texture_id) = node.texture_id.load().as_ref() {
            let texture = payload.resource_manager.get_texture(&texture_id);

            if TextureStatus::Ready != texture.status() {
                return;
            }

            let (tex_width, tex_height) = texture.size();

            let width = (tex_width as f64 * scale_factor) / (logical_width * scale_factor) * 2.;
            let height =
                (tex_height as f64 * scale_factor) / (logical_height * scale_factor) as f64 * 2.;

            if node.base_mut().pop_update_vertices() {
                let vertices = calculate_rect_vertices(node, width, height, &node.area);

                if node.vertex_buffer.is_none() {
                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Vertex Buffer"),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        });

                    node.vertex_buffer = Some(vertex_buffer);
                } else {
                    let buf = bytemuck::cast_slice(&vertices);
                    staging_belt
                        .write_buffer(
                            encoder,
                            node.vertex_buffer.as_ref().unwrap(),
                            0,
                            (buf.len() as u64).try_into().unwrap(),
                            &device,
                        )
                        .copy_from_slice(buf);
                }
            }

            // create bind group if not exist
            if !self.bind_group_map.contains_key(texture_id) {
                let bind_group = self.get_bind_group(device, &texture);
                self.bind_group_map.insert(texture_id.clone(), bind_group);
            }
        }
    }

    fn begin(&self) {}
    fn finish(&self) {}

    fn render<'a, 'b: 'a>(
        &'b self,
        _: &Arc<Device>,
        _: &Arc<Queue>,
        render_pass: &mut RenderPass<'a>,
        node: &'b dyn Node,
    ) {
        let mut bind_group = None;
        let mut vertex_buffer = None;

        if let Some(sprite) = node.as_any().downcast_ref::<Sprite>() {
            if let Some(texture_id) = sprite.texture_id.load().as_ref() {
                bind_group = self.bind_group_map.get(texture_id);
                vertex_buffer = sprite.vertex_buffer.as_ref();
            }
        }
        // else if let Some(video) = node.as_any().downcast_ref::<Video>() {
        //     bind_group = video.texture.read().bind_group.as_ref().unwrap().clone();
        //     vertex_buffer = video.vertex_buffer.as_ref().unwrap();
        // }
        else {
            unreachable!()
        }

        if bind_group.is_some() && vertex_buffer.is_some() {
            render_pass.set_pipeline(self.render_pipeline());
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.set_bind_group(0, bind_group.unwrap(), &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.unwrap().slice(..));

            // FIXME: NUM_INDICES depends on which renderer the child matches.
            render_pass.draw_indexed(0..NUM_INDICES, 0, 0..1);
        }
    }
}
