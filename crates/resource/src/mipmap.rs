use wgpu::*;

#[derive(Debug)]
pub(crate) struct MipmapGenerator {
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl MipmapGenerator {
    pub(crate) fn new(device: &Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Mipmap Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Mipmap Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Mipmap Shader"),
            source: ShaderSource::Wgsl(include_str!("mipmap.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Mipmap Pipeline"),
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
                    format: TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            bind_group_layout,
            pipeline,
        }
    }

    pub(crate) fn generate(
        &self,
        device: &Device,
        queue: &Queue,
        texture: &Texture,
        mip_level_count: u32,
    ) {
        if mip_level_count <= 1 {
            return;
        }

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Mipmap Command Encoder"),
        });

        for mip_level in 1..mip_level_count {
            let source_view = texture.create_view(&TextureViewDescriptor {
                label: Some("Mipmap Source View"),
                base_mip_level: mip_level - 1,
                mip_level_count: Some(1),
                ..Default::default()
            });
            let destination_view = texture.create_view(&TextureViewDescriptor {
                label: Some("Mipmap Destination View"),
                base_mip_level: mip_level,
                mip_level_count: Some(1),
                ..Default::default()
            });
            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("Mipmap Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&source_view),
                }],
            });
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Mipmap Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &destination_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}
