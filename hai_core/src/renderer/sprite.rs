use wgpu::{util::DeviceExt, *};

use crate::{nodes::SPRITE_INDICES, traits::Renderer, types::Vertex};

/// the number of vertices in a sprite is always 4.
// pub static NUM_VERTICES: u32 = 4;

pub static NUM_INDICES: u32 = SPRITE_INDICES.len() as u32;

pub struct SpriteRenderer {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    index_buffer: Buffer,
}

impl SpriteRenderer {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
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
            label: Some("Sprite Renderer Index Buffer"),
            contents: bytemuck::cast_slice(SPRITE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            pipeline,
            bind_group_layout,
            index_buffer,
        }
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

    fn index_buffer(&self) -> &Buffer {
        &self.index_buffer
    }
}
