use moyu_core::base::MVPMatrix;
use moyu_core::utils::coordinates::calculate_surface_physical_coordinates;
use wgpu::util::DeviceExt;
use wgpu::*;

use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::{
    Node, NodeBaseTrait, RenderCommandSender, Renderer, RendererUpdatePayload,
};

use crate::nodes::Filter;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FilterParams {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub tint: [f32; 4],
}

pub struct OffscreenPassRenderer {
    format: TextureFormat,
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
}

impl OffscreenPassRenderer {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Backdrop Shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/filter.wgsl").into()),
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

impl Renderer for OffscreenPassRenderer {
    fn name(&self) -> &'static str {
        "filter"
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        unreachable!("OffscreenPassRenderer does not use a render pipeline")
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        unreachable!("OffscreenPassRenderer does not use a bind group layout")
    }

    fn update(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        _queue: &Queue,
        render_queue: &RenderCommandSender,
        payload: &RendererUpdatePayload,
    ) {
        let filter = node.as_any_mut().downcast_mut::<Filter>().unwrap();

        // Calculate the bounding box of the node by transforming its local bounds
        // to stage logical coordinates and clipping to stage dimensions.
        let bounds = filter
            .base()
            .content_bounds()
            .transform(filter.base().global_transform());

        let bounds = bounds.clamp(
            0.0,
            0.0,
            payload.stage_logical_size.0 as f32,
            payload.stage_logical_size.1 as f32,
        );

        if bounds.max_x() <= bounds.min_x() || bounds.max_y() <= bounds.min_y() {
            filter.rect = None;
            return;
        }

        let rect = bounds.into_rect();

        let (_, _, width, height) = calculate_surface_physical_coordinates(
            &rect,
            payload.stage_logical_size,
            payload.surface_logical_size,
            payload.scale_factor,
        );

        // Check if we need to create or recreate textures
        let needs_recreation = filter.offscreen_view.is_none()
            || filter.last_width != width
            || filter.last_height != height;

        if needs_recreation && width > 0 && height > 0 {
            let offscreen_texture = device.create_texture(&TextureDescriptor {
                label: Some("Filter Offscreen Texture"),
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
            let offscreen_view = offscreen_texture.create_view(&TextureViewDescriptor::default());

            // Create final texture for filter results
            let final_texture = device.create_texture(&TextureDescriptor {
                label: Some("Filter Final Texture"),
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

            let params = FilterParams {
                position: [rect.x(), rect.y()],
                size: [rect.width(), rect.height()],
                tint: [
                    filter.base().tint().r,
                    filter.base().tint().g,
                    filter.base().tint().b,
                    filter.base().tint().a * filter.base().global_opacity(),
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
            filter.offscreen_view = Some(offscreen_view);
            filter.final_view = Some(final_view);
            filter.rect = Some(rect);
            filter.buffer = Some(params_buffer);
            filter.bind_group = Some(bind_group);
            filter.last_width = width;
            filter.last_height = height;
        } else {
            filter.rect = Some(rect);

            if let Some(params_buffer) = &filter.buffer {
                let params = FilterParams {
                    position: [rect.x(), rect.y()],
                    size: [rect.width(), rect.height()],
                    tint: [
                        filter.base().tint().r,
                        filter.base().tint().g,
                        filter.base().tint().b,
                        filter.base().tint().a * filter.base().global_opacity(),
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
        let filter = node.as_any().downcast_ref::<Filter>().unwrap();

        // 获取离屏纹理引用
        let (offscreen_view, rect) = match (filter.offscreen_view.as_ref(), filter.rect.as_ref()) {
            (Some(view), Some(rect)) => (view.clone(), rect.clone()),
            _ => {
                // 纹理未初始化，跳过
                return;
            }
        };

        render_queue
            .send(RenderCommand::BeginOffscreenPass {
                offscreen_view,
                rect,
            })
            .unwrap();
    }

    fn collect_post_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        let filter = node
            .as_any()
            .downcast_ref::<Filter>()
            .expect("Node is not OffscreenPass");

        // 获取所有纹理引用
        let (offscreen_view, final_view, rect) = match (
            filter.offscreen_view.as_ref(),
            filter.final_view.as_ref(),
            filter.rect.as_ref(),
        ) {
            (Some(ov), Some(fv), Some(rect)) => (ov.clone(), fv.clone(), rect.clone()),
            _ => {
                // 纹理未初始化，跳过
                return;
            }
        };

        render_queue
            .send(RenderCommand::EndOffscreenPass {
                offscreen_view,
                final_view,
                rect,
                filters: filter.filters.clone(),
            })
            .unwrap();

        render_queue
            .send(RenderCommand::Draw {
                pipeline: self.pipeline.clone(),
                bind_group: filter.bind_group.clone().unwrap(),
                extra_bind_groups: vec![],
                vertex_buffer: None,
                index_buffer: None,
                instance_buffer: None,
                count: 6,
                instance_count: 1,
            })
            .unwrap();
    }
}
