use crate::core::render_command::FilterKind;
use crate::core::texture_pool::TexturePool;
use crate::traits::FilterRenderer;
use wgpu::*;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurParams {
    texel_size: [f32; 2],
    blur_radius: f32,
    _padding: f32,
}

const INITIAL_CAPACITY: u64 = 16;

pub struct BlurPerfectFilterRenderer {
    horizontal_pipeline: RenderPipeline,
    vertical_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    format: TextureFormat,
    uniform_buffer: Buffer,
    frame_offset: u64,
    buffer_capacity: u64,
    alignment: u64,
}

impl BlurPerfectFilterRenderer {
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        let horizontal_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Blur Horizontal Shader"),
            source: ShaderSource::Wgsl(include_str!("blur_horizontal.wgsl").into()),
        });

        let vertical_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Blur Vertical Shader"),
            source: ShaderSource::Wgsl(include_str!("blur_vertical.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Blur Bind Group Layout"),
            entries: &[
                // Source texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // BlurParams uniform
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Blur Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let horizontal_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Blur Horizontal Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &horizontal_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &horizontal_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let vertical_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Blur Vertical Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vertical_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &vertical_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Blur Sampler"),
            // Use MirrorRepeat to avoid edge artifacts
            // See https://chromestatus.com/feature/5382638738341888
            address_mode_u: AddressMode::MirrorRepeat,
            address_mode_v: AddressMode::MirrorRepeat,
            address_mode_w: AddressMode::MirrorRepeat,
            // Must be linear for fast gaussian blur algorithm
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });

        let alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let buffer_capacity = INITIAL_CAPACITY;

        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Blur Perfect Uniform Buffer"),
            size: alignment * buffer_capacity,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            horizontal_pipeline,
            vertical_pipeline,
            bind_group_layout,
            sampler,
            format,
            uniform_buffer,
            frame_offset: 0,
            buffer_capacity,
            alignment,
        }
    }
}

impl FilterRenderer for BlurPerfectFilterRenderer {
    fn name(&self) -> &'static str {
        "blur-perfect"
    }

    fn execute(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        input: &TextureView,
        output: &TextureView,
        filter: &FilterKind,
        width: u32,
        height: u32,
        scale: f32,
        pool: &mut TexturePool,
        timestamp: f64,
    ) {
        let FilterKind::BlurPerfect { radius } = filter else {
            return;
        };
        if *radius <= 0.0 {
            return;
        }

        let radius = radius * scale;

        // Check if we need to expand the buffer
        if self.frame_offset + self.alignment > self.alignment * self.buffer_capacity {
            let new_capacity = self.buffer_capacity * 2;
            log::warn!(
                "[BlurPerfect] Uniform buffer capacity exceeded ({} slots), expanding to {} slots",
                self.buffer_capacity,
                new_capacity
            );

            self.uniform_buffer = device.create_buffer(&BufferDescriptor {
                label: Some("Blur Perfect Uniform Buffer (Expanded)"),
                size: self.alignment * new_capacity,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.buffer_capacity = new_capacity;
        }

        // Acquire from pool
        let intermediate_pooled = pool.acquire(device, width, height, self.format, timestamp);
        let intermediate_view = &intermediate_pooled.view;

        let blur_params = BlurParams {
            texel_size: [1.0 / width as f32, 1.0 / height as f32],
            blur_radius: radius,
            _padding: 0.0,
        };

        // Get current offset and increment
        let offset = self.frame_offset;
        self.frame_offset += self.alignment;

        // Write to buffer at offset
        queue.write_buffer(
            &self.uniform_buffer,
            offset,
            bytemuck::bytes_of(&blur_params),
        );

        // Pass 1: Horizontal Blur (input -> intermediate)
        {
            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("Horizontal Blur Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(input),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: &self.uniform_buffer,
                            offset,
                            size: Some(
                                std::num::NonZeroU64::new(std::mem::size_of::<BlurParams>() as u64)
                                    .unwrap(),
                            ),
                        }),
                    },
                ],
            });

            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Horizontal Blur Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &intermediate_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.horizontal_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        // Pass 2: Vertical Blur (intermediate -> output)
        {
            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("Vertical Blur Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&intermediate_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: &self.uniform_buffer,
                            offset,
                            size: Some(
                                std::num::NonZeroU64::new(std::mem::size_of::<BlurParams>() as u64)
                                    .unwrap(),
                            ),
                        }),
                    },
                ],
            });

            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Vertical Blur Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: output,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.vertical_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        pool.return_texture(intermediate_pooled);
    }

    fn reset_frame(&mut self) {
        self.frame_offset = 0;
    }
}
