use std::sync::Arc;

use glam::Vec2;
use hai_pal::env::get_hai_env;
use huozi::constant::TEXTURE_SIZE;
use huozi::layout::Vertex;
use huozi::Huozi;
use log::error;
use wgpu::util::StagingBelt;
use wgpu::Texture;
use wgpu::{util::DeviceExt, *};

use crate::base::MVPMatrix;
use crate::nodes::Text;
use crate::traits::Renderer;
use crate::traits::{Node, NodeBaseTrait, RendererUpdatePayload};

/// the number of vertices in a sprite is always 4.
// pub static NUM_VERTICES: u32 = 4;

pub struct TextRenderer {
    pipeline: RenderPipeline,
    texture: Texture,
    _sampler: Sampler,
    _view: TextureView,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    huozi: Huozi,
}

impl TextRenderer {
    pub fn new(device: &Arc<Device>, config: &SurfaceConfiguration) -> Self {
        let size = wgpu::Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_SIZE,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sdf texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &vec![],
        });

        let _view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let _sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("text_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&_sampler),
                },
            ],
            label: Some("text_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/text.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[&MVPMatrix::bind_group_layout(device), &bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Text Render Pipeline"),
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

        let font_data = std::fs::read(&get_hai_env().font_file).unwrap();
        let huozi = Huozi::new(font_data);

        Self {
            pipeline,
            texture,
            _sampler,
            _view,
            bind_group_layout,
            bind_group,
            huozi,
        }
    }
}

impl TextRenderer {}

impl Renderer for TextRenderer {
    fn name(&self) -> &'static str {
        return "text";
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
        queue: &Arc<Queue>,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
        _: &RendererUpdatePayload,
    ) {
        let node = node.as_any_mut().downcast_mut::<Text>().unwrap();

        if node.base_mut().pop_update_vertices() {
            match self
                .huozi
                .layout_parse(&node.text, &node.layout_style, &node.text_style, None)
            {
                Ok((mut vertices, indices, total_width, total_height)) => {
                    // set layout size
                    node.total_width = total_width;
                    node.total_height = total_height;

                    // transform to global
                    let transform = node.base().global_transform();
                    for vertex in vertices.iter_mut() {
                        // FIXME: convertion between Vec2 and [f32; 2] may cause additional cost
                        // y axis is inverted, so we need to invert it back, apply transform and invert it again
                        let p = transform
                            .transform_point2(Vec2::new(vertex.position[0], vertex.position[1]));
                        vertex.position[0] = p.x;
                        vertex.position[1] = p.y;
                    }

                    if node.vertex_buffer.is_none() {
                        let vertex_buffer =
                            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("Text Vertex Buffer"),
                                contents: bytemuck::cast_slice(&vertices),
                                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            });
                        let index_buffer =
                            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("Index Buffer"),
                                contents: bytemuck::cast_slice(&indices),
                                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                            });
                        let num_indices = indices.len() as u32;

                        node.vertex_buffer = Some(vertex_buffer);
                        node.index_buffer = Some(index_buffer);
                        node.num_indices = num_indices;
                    } else {
                        let buf_vertices = bytemuck::cast_slice(&vertices);
                        let buf_indices = bytemuck::cast_slice(&indices);
                        staging_belt
                            .write_buffer(
                                encoder,
                                node.vertex_buffer.as_ref().unwrap(),
                                0,
                                (buf_vertices.len() as u64).try_into().unwrap(),
                                &device,
                            )
                            .copy_from_slice(buf_vertices);
                        staging_belt
                            .write_buffer(
                                encoder,
                                node.index_buffer.as_ref().unwrap(),
                                0,
                                (buf_indices.len() as u64).try_into().unwrap(),
                                &device,
                            )
                            .copy_from_slice(buf_indices);
                    }

                    // update sdf texture
                    let sdf_bitmap = self.huozi.texture_image();
                    let dimensions = sdf_bitmap.dimensions();

                    let size = wgpu::Extent3d {
                        width: dimensions.0,
                        height: dimensions.1,
                        depth_or_array_layers: 1,
                    };

                    queue.write_texture(
                        wgpu::ImageCopyTexture {
                            aspect: wgpu::TextureAspect::All,
                            texture: &self.texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                        },
                        &sdf_bitmap,
                        wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * sdf_bitmap.width()),
                            rows_per_image: Some(sdf_bitmap.height()),
                        },
                        size,
                    );
                }
                Err(err_msg) => {
                    error!("{}", err_msg);
                }
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
        if !node.base().visible() {
            return;
        }

        let node = node
            .as_any()
            .downcast_ref::<Text>()
            .expect("this node is not a text node");

        if node.vertex_buffer.is_some() && node.index_buffer.is_some() {
            let vertex_buffer = node.vertex_buffer.as_ref().unwrap();
            let index_buffer = node.index_buffer.as_ref().unwrap();
            let num_indices = node.num_indices;

            render_pass.set_pipeline(self.render_pipeline());
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.set_bind_group(1, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

            // FIXME: NUM_INDICES depends on which renderer the child matches.
            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        }
    }
}
