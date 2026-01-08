use crate::core::render_command::FilterKind;
use crate::traits::FilterRenderer;
use wgpu::util::DeviceExt;
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
        }
    }
}

impl FilterRenderer for BlurFilterRenderer {
    fn name(&self) -> &'static str {
        "blur"
    }

    fn execute(
        &self,
        device: &Device,
        encoder: &mut CommandEncoder,
        input: &TextureView,
        output: &TextureView,
        filter: &FilterKind,
        width: u32,
        height: u32,
    ) {
        let FilterKind::Blur { radius, continuous } = filter else {
            return;
        };

        let radius = *radius;
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

        let mut textures = Vec::new();
        let mut downsampled_views = Vec::new();
        downsampled_views.push(input.clone());

        let max_scale = levels[idx2].1;
        let mut current_scale = 1;
        let mut current_view = input.clone();

        while current_scale < max_scale {
            let next_scale = current_scale * 2;
            let sw = (width / next_scale).max(1);
            let sh = (height / next_scale).max(1);

            let ds_tex = device.create_texture(&TextureDescriptor {
                label: Some("Blur Iterative Downsample Texture"),
                size: Extent3d {
                    width: sw,
                    height: sh,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: self.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let ds_view = ds_tex.create_view(&TextureViewDescriptor::default());

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
            textures.push(ds_tex);
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
            let h_tex = device.create_texture(&TextureDescriptor {
                label: Some("Blur Horizontal Texture"),
                size: Extent3d {
                    width: sw,
                    height: sh,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: self.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let h_view = h_tex.create_view(&TextureViewDescriptor::default());

            let blur_params = BlurParams {
                texel_size: [1.0 / sw as f32, 1.0 / sh as f32],
                blur_radius: adjusted_radius,
                _padding: 0.0,
            };
            let blur_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Blur Params Buffer"),
                contents: bytemuck::bytes_of(&blur_params),
                usage: BufferUsages::UNIFORM,
            });

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
                        resource: blur_params_buffer.as_entire_binding(),
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
            let v_tex = device.create_texture(&TextureDescriptor {
                label: Some("Blur Vertical Texture"),
                size: Extent3d {
                    width: sw,
                    height: sh,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: self.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let v_view = v_tex.create_view(&TextureViewDescriptor::default());

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
                        resource: blur_params_buffer.as_entire_binding(),
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

            textures.push(h_tex);
            textures.push(v_tex);
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
            let blend_params_buffer =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Blur Blend Params Buffer"),
                    contents: bytemuck::bytes_of(&blend_params),
                    usage: BufferUsages::UNIFORM,
                });

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
                        resource: blend_params_buffer.as_entire_binding(),
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
    }
}
