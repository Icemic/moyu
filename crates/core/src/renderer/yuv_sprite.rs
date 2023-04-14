use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::StagingBelt;
use wgpu::{util::DeviceExt, *};

use crate::nodes::{Texture, YUVSprite};
use crate::resource::TextureId;
use crate::traits::{Node, RendererUpdatePayload};
use crate::utils::calculate::calculate_rect_vertices;
use crate::{traits::Renderer, types::Vertex, utils::constants::RECTANGLE_INDICES};

/// the number of vertices in a sprite is always 4.
// pub static NUM_VERTICES: u32 = 4;

pub static NUM_INDICES: u32 = RECTANGLE_INDICES.len() as u32;

pub struct YUVSpriteRenderer {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    index_buffer: Buffer,
    bind_group_map: HashMap<Arc<TextureId>, BindGroup>,
}

impl YUVSpriteRenderer {
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
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: None,
        });

        // shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(include_str!("./shaders/i420.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("YUV Sprite Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
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
            label: Some("YUV Sprite Renderer Index Buffer"),
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

impl YUVSpriteRenderer {
    fn get_bind_group(
        &mut self,
        device: &Device,
        texture_y: &Texture,
        texture_u: &Texture,
        texture_v: &Texture,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        texture_y.view.load().as_ref().unwrap(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(
                        texture_y.sampler.load().as_ref().unwrap(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        texture_u.view.load().as_ref().unwrap(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(
                        texture_u.sampler.load().as_ref().unwrap(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(
                        texture_v.view.load().as_ref().unwrap(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(
                        texture_v.sampler.load().as_ref().unwrap(),
                    ),
                },
            ],
            label: None,
        })
    }
}

impl Renderer for YUVSpriteRenderer {
    fn name(&self) -> &'static str {
        return "yuv_sprite";
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    fn index_buffer(&self) -> &Buffer {
        &self.index_buffer
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

        let mut node = node.as_any_mut().downcast_mut::<YUVSprite>().unwrap();

        if let Some(texture_id) = node.texture_id.load().as_ref() {
            let textures = node.textures.load();
            let textures = textures.as_ref().expect("textures must be set.");
            let (texture_y, texture_u, texture_v) = &**textures;

            let (tex_width, tex_height) = texture_y.size();

            let width = (tex_width as f64 * scale_factor) / (logical_width * scale_factor) * 2.;
            let height =
                (tex_height as f64 * scale_factor) / (logical_height * scale_factor) as f64 * 2.;

            let vertices = calculate_rect_vertices(node, width, height, &node.area);

            if node.vertex_buffer.is_none() {
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

            // create bind group if not exist
            if !self.bind_group_map.contains_key(texture_id) {
                let bind_group = self.get_bind_group(device, texture_y, texture_u, texture_v);
                self.bind_group_map.insert(texture_id.clone(), bind_group);
            }
        }
    }

    fn begin(&self) {}
    fn finish(&self) {}

    fn render<'a, 'b: 'a>(&'b self, render_pass: &mut RenderPass<'a>, node: &'b dyn Node) {
        let mut bind_group = None;
        let mut vertex_buffer = None;

        if let Some(sprite) = node.as_any().downcast_ref::<YUVSprite>() {
            if let Some(texture_id) = sprite.texture_id.load().as_ref() {
                bind_group = self.bind_group_map.get(texture_id);
                vertex_buffer = sprite.vertex_buffer.as_ref();
            }
        } else {
            unreachable!()
        }

        if bind_group.is_some() && vertex_buffer.is_some() {
            render_pass.set_pipeline(self.render_pipeline());
            render_pass.set_index_buffer(self.index_buffer().slice(..), wgpu::IndexFormat::Uint16);

            render_pass.set_bind_group(0, bind_group.unwrap(), &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.unwrap().slice(..));

            // FIXME: NUM_INDICES depends on which renderer the child matches.
            render_pass.draw_indexed(0..NUM_INDICES, 0, 0..1);
        }
    }
}
