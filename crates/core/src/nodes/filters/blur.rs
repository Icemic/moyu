use crate::core::render_command::FilterKind;
use crate::core::texture_pool::{PooledTexture, TexturePool};
use crate::traits::FilterRenderer;
use wgpu::*;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurParams {
    texel_size: [f32; 2],
    blur_radius: f32,
    _padding: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BlendParams {
    weight: f32,
    _padding: [f32; 3],
}

const INITIAL_CAPACITY: u64 = 16;

pub struct BlurFilterRenderer {
    horizontal_pipeline: RenderPipeline,
    vertical_pipeline: RenderPipeline,
    blit_pipeline: RenderPipeline,
    blend_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    blit_bind_group_layout: BindGroupLayout,
    blend_bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    format: TextureFormat,
    blur_uniform_buffer: Buffer,
    blend_uniform_buffer: Buffer,
    blur_frame_offset: u64,
    blend_frame_offset: u64,
    blur_buffer_capacity: u64,
    blend_buffer_capacity: u64,
    alignment: u64,
}

impl BlurFilterRenderer {
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        let horizontal_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Blur Horizontal Shader"),
            source: ShaderSource::Wgsl(include_str!("blur_horizontal.wgsl").into()),
        });

        let vertical_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Blur Vertical Shader"),
            source: ShaderSource::Wgsl(include_str!("blur_vertical.wgsl").into()),
        });

        let blit_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Blur Blit Shader"),
            source: ShaderSource::Wgsl(include_str!("blit.wgsl").into()),
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

        let blit_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Blur Blit Bind Group Layout"),
            entries: &[
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
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let blend_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Blur Blend Bind Group Layout"),
            entries: &[
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
                BindGroupLayoutEntry {
                    binding: 1,
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
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Blur Blit Pipeline Layout"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });

        let blend_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Blur Blend Pipeline Layout"),
            bind_group_layouts: &[&blit_bind_group_layout, &blend_bind_group_layout],
            push_constant_ranges: &[],
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
            multiview: None,
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
            multiview: None,
            cache: None,
        });

        let blit_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Blur Blit Pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: VertexState {
                module: &blit_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &blit_shader,
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
            multiview: None,
            cache: None,
        });

        let blend_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Blur Blend Pipeline"),
            layout: Some(&blend_pipeline_layout),
            vertex: VertexState {
                module: &blit_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_blend"),
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
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Blur Sampler"),
            address_mode_u: AddressMode::MirrorRepeat,
            address_mode_v: AddressMode::MirrorRepeat,
            address_mode_w: AddressMode::MirrorRepeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let blur_buffer_capacity = INITIAL_CAPACITY;
        let blend_buffer_capacity = INITIAL_CAPACITY;

        let blur_uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Blur Uniform Buffer"),
            size: alignment * blur_buffer_capacity,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let blend_uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Blur Blend Uniform Buffer"),
            size: alignment * blend_buffer_capacity,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            horizontal_pipeline,
            vertical_pipeline,
            blit_pipeline,
            blend_pipeline,
            bind_group_layout,
            blit_bind_group_layout,
            blend_bind_group_layout,
            sampler,
            format,
            blur_uniform_buffer,
            blend_uniform_buffer,
            blur_frame_offset: 0,
            blend_frame_offset: 0,
            blur_buffer_capacity,
            blend_buffer_capacity,
            alignment,
        }
    }
}

impl FilterRenderer for BlurFilterRenderer {
    fn name(&self) -> &'static str {
        "blur"
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
        let FilterKind::Blur { radius, continuous } = filter else {
            return;
        };

        let radius = *radius * scale;
        let continuous = continuous.unwrap_or(false);

        if radius <= 0.0 {
            return;
        }

        let levels = [(0.0, 1), (6.0, 2), (12.0, 4), (24.0, 8), (48.0, 16)];

        let mut idx1 = 0;
        let mut idx2 = 0;
        let mut weight = 0.0;

        if !continuous {
            for (i, &(t, _)) in levels.iter().enumerate() {
                if radius >= t {
                    idx1 = i;
                }
            }
            idx2 = idx1;
        } else {
            for i in 0..levels.len() - 1 {
                let t_next = levels[i + 1].0;
                let margin = (t_next * 0.2).min(5.0);
                if radius < t_next - margin {
                    idx1 = i;
                    idx2 = i;
                    break;
                } else if radius < t_next + margin {
                    idx1 = i;
                    idx2 = i + 1;
                    weight = (radius - (t_next - margin)) / (margin * 2.0);
                    break;
                }
                if i == levels.len() - 2 {
                    idx1 = i + 1;
                    idx2 = i + 1;
                }
            }
        }

        let mut pooled_resources: Vec<PooledTexture> = Vec::new();
        let mut downsampled_views = Vec::new();
        downsampled_views.push(input.clone());

        let max_scale = levels[idx2].1;
        let mut current_scale = 1;
        let mut current_view = input.clone();

        while current_scale < max_scale {
            let next_scale = current_scale * 2;
            let sw = (width / next_scale).max(1);
            let sh = (height / next_scale).max(1);

            let ds_pooled = pool.acquire(device, sw, sh, self.format, timestamp);
            let ds_view = ds_pooled.view.clone();

            let blit_bind_group = device.create_bind_group(&BindGroupDescriptor {
                layout: &self.blit_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&current_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                ],
                label: None,
            });

            {
                let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Blur Iterative Downsample Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &ds_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(&self.blit_pipeline);
                pass.set_bind_group(0, &blit_bind_group, &[]);
                pass.draw(0..6, 0..1);
            }

            current_view = ds_view.clone();
            downsampled_views.push(ds_view);
            pooled_resources.push(ds_pooled);
            current_scale = next_scale;
        }

        let mut render_level = |level_idx: usize, r: f32| -> TextureView {
            let scale = levels[level_idx].1;
            let sw = (width / scale).max(1);
            let sh = (height / scale).max(1);
            let adjusted_radius = r / scale as f32;
            let ds_view = &downsampled_views[level_idx];

            if adjusted_radius <= 0.1 {
                return ds_view.clone();
            }

            // 1. Horizontal Blur
            let h_pooled = pool.acquire(device, sw, sh, self.format, timestamp);
            let h_view = h_pooled.view.clone();

            let blur_params = BlurParams {
                texel_size: [1.0 / sw as f32, 1.0 / sh as f32],
                blur_radius: adjusted_radius,
                _padding: 0.0,
            };

            // Check if we need to expand the blur buffer
            if self.blur_frame_offset + self.alignment > self.alignment * self.blur_buffer_capacity
            {
                let new_capacity = self.blur_buffer_capacity * 2;
                log::warn!(
                    "[Blur] Blur uniform buffer capacity exceeded ({} slots), expanding to {} slots",
                    self.blur_buffer_capacity,
                    new_capacity
                );

                self.blur_uniform_buffer = device.create_buffer(&BufferDescriptor {
                    label: Some("Blur Uniform Buffer (Expanded)"),
                    size: self.alignment * new_capacity,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.blur_buffer_capacity = new_capacity;
            }

            // Get current offset and increment
            let blur_offset = self.blur_frame_offset;
            self.blur_frame_offset += self.alignment;

            // Write to buffer at offset
            queue.write_buffer(
                &self.blur_uniform_buffer,
                blur_offset,
                bytemuck::bytes_of(&blur_params),
            );

            let h_bind_group = device.create_bind_group(&BindGroupDescriptor {
                layout: &self.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(ds_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: &self.blur_uniform_buffer,
                            offset: blur_offset,
                            size: Some(
                                std::num::NonZeroU64::new(std::mem::size_of::<BlurParams>() as u64)
                                    .unwrap(),
                            ),
                        }),
                    },
                ],
                label: None,
            });

            {
                let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Blur Horizontal Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &h_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(&self.horizontal_pipeline);
                pass.set_bind_group(0, &h_bind_group, &[]);
                pass.draw(0..6, 0..1);
            }

            // 2. Vertical Blur
            let v_pooled = pool.acquire(device, sw, sh, self.format, timestamp);
            let v_view = v_pooled.view.clone();

            let v_bind_group = device.create_bind_group(&BindGroupDescriptor {
                layout: &self.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&h_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: &self.blur_uniform_buffer,
                            offset: blur_offset,
                            size: Some(
                                std::num::NonZeroU64::new(std::mem::size_of::<BlurParams>() as u64)
                                    .unwrap(),
                            ),
                        }),
                    },
                ],
                label: None,
            });

            {
                let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Blur Vertical Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &v_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(&self.vertical_pipeline);
                pass.set_bind_group(0, &v_bind_group, &[]);
                pass.draw(0..6, 0..1);
            }

            pooled_resources.push(h_pooled);
            pooled_resources.push(v_pooled);
            v_view
        };

        let view1 = render_level(idx1, radius);

        if idx1 == idx2 {
            // Final Upsample
            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                layout: &self.blit_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&view1),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                ],
                label: None,
            });

            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Blur Final Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: output,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.blit_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..6, 0..1);
        } else {
            let view2 = render_level(idx2, radius);

            let blend_params = BlendParams {
                weight,
                _padding: [0.0; 3],
            };

            // Check if we need to expand the blend buffer
            if self.blend_frame_offset + self.alignment
                > self.alignment * self.blend_buffer_capacity
            {
                let new_capacity = self.blend_buffer_capacity * 2;
                log::warn!(
                    "[Blur] Blend uniform buffer capacity exceeded ({} slots), expanding to {} slots",
                    self.blend_buffer_capacity,
                    new_capacity
                );

                self.blend_uniform_buffer = device.create_buffer(&BufferDescriptor {
                    label: Some("Blur Blend Uniform Buffer (Expanded)"),
                    size: self.alignment * new_capacity,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.blend_buffer_capacity = new_capacity;
            }

            // Get current offset and increment
            let blend_offset = self.blend_frame_offset;
            self.blend_frame_offset += self.alignment;

            // Write to buffer at offset
            queue.write_buffer(
                &self.blend_uniform_buffer,
                blend_offset,
                bytemuck::bytes_of(&blend_params),
            );

            let bind_group0 = device.create_bind_group(&BindGroupDescriptor {
                layout: &self.blit_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&view1),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                ],
                label: None,
            });

            let bind_group1 = device.create_bind_group(&BindGroupDescriptor {
                layout: &self.blend_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&view2),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: &self.blend_uniform_buffer,
                            offset: blend_offset,
                            size:
                                Some(
                                    std::num::NonZeroU64::new(
                                        std::mem::size_of::<BlendParams>() as u64
                                    )
                                    .unwrap(),
                                ),
                        }),
                    },
                ],
                label: None,
            });

            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Blur Final Blend Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: output,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.blend_pipeline);
            pass.set_bind_group(0, &bind_group0, &[]);
            pass.set_bind_group(1, &bind_group1, &[]);
            pass.draw(0..6, 0..1);
        }

        for t in pooled_resources {
            pool.return_texture(t);
        }
    }

    fn reset_frame(&mut self) {
        self.blur_frame_offset = 0;
        self.blend_frame_offset = 0;
    }
}
