use moyu_core::base::MVPMatrix;
use moyu_core::utils::coordinates::calculate_surface_physical_coordinates;
use wgpu::util::DeviceExt;
use wgpu::*;

use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::{
    Node, NodeBaseTrait, RenderCommandSender, Renderer, RendererUpdatePayload,
};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BackdropParams {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub tint: [f32; 4],
}

pub struct BackdropRenderer {
    format: TextureFormat,
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
}

impl BackdropRenderer {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Backdrop Shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/backdrop.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Backdrop Bind Group Layout"),
            entries: &[
                // BackdropParams uniform
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
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
                // Texture
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Backdrop Pipeline Layout"),
            bind_group_layouts: &[&MVPMatrix::bind_group_layout(device), &bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Backdrop Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
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
            label: Some("Backdrop Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        Self {
            format: config.format,
            pipeline,
            bind_group_layout,
            sampler,
        }
    }
}

impl Renderer for BackdropRenderer {
    fn name(&self) -> &'static str {
        "backdrop"
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        unreachable!("BackdropRenderer does not use a render pipeline")
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        unreachable!("BackdropRenderer does not use a bind group layout")
    }

    fn update(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        _queue: &Queue,
        render_queue: &RenderCommandSender,
        payload: &RendererUpdatePayload,
    ) {
        use crate::nodes::backdrop::Backdrop;

        let backdrop = node.as_any_mut().downcast_mut::<Backdrop>().unwrap();

        let rect = backdrop
            .base()
            .bounds()
            .transform(backdrop.base().global_transform())
            .into_rect();

        let (_, _, width, height) = calculate_surface_physical_coordinates(
            &rect,
            payload.stage_logical_size,
            payload.surface_logical_size,
            payload.scale_factor,
        );

        // Check if we need to create or recreate textures
        let needs_recreation = backdrop.source_view.is_none()
            || backdrop.last_width != width
            || backdrop.last_height != height;

        if needs_recreation && width > 0 && height > 0 {
            // Create source texture
            let source_texture = device.create_texture(&TextureDescriptor {
                label: Some("Backdrop Source Texture"),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: self.format,
                usage: TextureUsages::RENDER_ATTACHMENT
                    | TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let source_view = source_texture.create_view(&TextureViewDescriptor::default());

            // Create final texture
            let final_texture = device.create_texture(&TextureDescriptor {
                label: Some("Backdrop Final Texture"),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: self.format,
                usage: TextureUsages::RENDER_ATTACHMENT
                    | TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let final_view = final_texture.create_view(&TextureViewDescriptor::default());

            let params = BackdropParams {
                position: [rect.x(), rect.y()],
                size: [rect.width(), rect.height()],
                tint: [
                    backdrop.base().tint().r,
                    backdrop.base().tint().g,
                    backdrop.base().tint().b,
                    backdrop.base().tint().a * backdrop.base().global_opacity(),
                ],
            };

            let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Blit Params Buffer"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("Blit Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: params_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&final_view),
                    },
                ],
            });

            // TODO: Call .destroy() on old textures to free memory immediately?
            // But currently there may be pending references in the render queue.
            backdrop.source_view = Some(source_view);
            backdrop.final_view = Some(final_view);
            backdrop.buffer = Some(params_buffer);
            backdrop.bind_group = Some(bind_group);
            backdrop.rect = Some(rect);
            backdrop.last_width = width;
            backdrop.last_height = height;
        } else {
            backdrop.rect = Some(rect);

            if let Some(params_buffer) = &backdrop.buffer {
                let params = BackdropParams {
                    position: [rect.x(), rect.y()],
                    size: [rect.width(), rect.height()],
                    tint: [
                        backdrop.base().tint().r,
                        backdrop.base().tint().g,
                        backdrop.base().tint().b,
                        backdrop.base().tint().a * backdrop.base().global_opacity(),
                    ],
                };

                let buf = bytemuck::bytes_of(&params);

                render_queue
                    .send(RenderCommand::WriteBuffer {
                        buffer: params_buffer.clone(),
                        offset: 0,
                        data: buf.to_vec(),
                        use_staging_belt: true,
                    })
                    .unwrap();
            }
        }
    }

    fn collect_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        use crate::nodes::backdrop::Backdrop;

        let backdrop = node.as_any().downcast_ref::<Backdrop>().unwrap();

        // 获取纹理引用
        let (source_view, final_view, rect) = match (
            backdrop.source_view.as_ref(),
            backdrop.final_view.as_ref(),
            backdrop.rect.as_ref(),
        ) {
            (Some(sv), Some(fv), Some(rect)) => (sv.clone(), fv.clone(), *rect),
            _ => {
                // 纹理未初始化，跳过渲染
                return;
            }
        };

        // Commit any previous commands before capturing backdrop
        render_queue.send(RenderCommand::Barrier).unwrap();

        // Capture the backdrop
        render_queue
            .send(RenderCommand::CaptureBackdrop {
                source_view: source_view.clone(),
                final_view: final_view.clone(),
                rect,
                filters: backdrop.filters.clone(),
            })
            .unwrap();

        render_queue
            .send(RenderCommand::Draw {
                pipeline: self.pipeline.clone(),
                bind_group: backdrop.bind_group.clone().unwrap(),
                extra_bind_groups: vec![],
                vertex_buffer: None,
                index_buffer: None,
                instance_buffer: None,
                count: 6,
            })
            .unwrap();
    }
}
