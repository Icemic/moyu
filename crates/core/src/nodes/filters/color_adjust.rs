use crate::core::render_command::FilterKind;
use crate::core::texture_pool::TexturePool;
use crate::traits::FilterRenderer;
use wgpu::*;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ColorAdjustParams {
    amount: f32,
    mode: u32,
    _padding: [f32; 2],
}

const MODE_BRIGHTNESS: u32 = 0;
const MODE_CONTRAST: u32 = 1;
const MODE_SATURATION: u32 = 2;
const MODE_HUE_ROTATE: u32 = 3;
const MODE_GRAYSCALE: u32 = 4;
const MODE_SEPIA: u32 = 5;
const MODE_INVERT: u32 = 6;

const INITIAL_CAPACITY: u64 = 16;

pub struct ColorAdjustFilterRenderer {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    uniform_buffer: Buffer,
    frame_offset: u64,
    buffer_capacity: u64,
    alignment: u64,
}

impl ColorAdjustFilterRenderer {
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Color Adjust Shader"),
            source: ShaderSource::Wgsl(include_str!("color_adjust.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Color Adjust Bind Group Layout"),
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
                // ColorAdjustParams uniform
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
            label: Some("Color Adjust Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Color Adjust Pipeline"),
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
            label: Some("Color Adjust Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });

        let alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let buffer_capacity = INITIAL_CAPACITY;

        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Color Adjust Uniform Buffer"),
            size: alignment * buffer_capacity,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            uniform_buffer,
            frame_offset: 0,
            buffer_capacity,
            alignment,
        }
    }
}

impl FilterRenderer for ColorAdjustFilterRenderer {
    fn name(&self) -> &'static str {
        "color-adjust"
    }

    fn execute(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        input: &TextureView,
        output: &TextureView,
        filter: &FilterKind,
        _width: u32,
        _height: u32,
        _scale: f32,
        _pool: &mut TexturePool,
        _timestamp: f64,
    ) {
        // Check if we need to expand the buffer
        if self.frame_offset + self.alignment > self.alignment * self.buffer_capacity {
            let new_capacity = self.buffer_capacity * 2;
            log::warn!(
                "[ColorAdjust] Uniform buffer capacity exceeded ({} slots), expanding to {} slots",
                self.buffer_capacity,
                new_capacity
            );

            self.uniform_buffer = device.create_buffer(&BufferDescriptor {
                label: Some("Color Adjust Uniform Buffer (Expanded)"),
                size: self.alignment * new_capacity,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.buffer_capacity = new_capacity;
        }

        let (amount, mode) = match filter {
            FilterKind::Brightness { amount } => (*amount, MODE_BRIGHTNESS),
            FilterKind::Contrast { amount } => (*amount, MODE_CONTRAST),
            FilterKind::Saturation { amount } => (*amount, MODE_SATURATION),
            FilterKind::HueRotate { degrees } => (*degrees, MODE_HUE_ROTATE),
            FilterKind::Grayscale { amount } => (*amount, MODE_GRAYSCALE),
            FilterKind::Sepia { amount } => (*amount, MODE_SEPIA),
            FilterKind::Invert { amount } => (*amount, MODE_INVERT),
            _ => (0.0, 999), // Should not happen given registry mapping
        };

        let params = ColorAdjustParams {
            amount,
            mode,
            _padding: [0.0; 2],
        };

        // Get current offset and increment
        let offset = self.frame_offset;
        self.frame_offset += self.alignment;

        // Write to buffer at offset
        queue.write_buffer(&self.uniform_buffer, offset, bytemuck::bytes_of(&params));

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Color Adjust Bind Group"),
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
                            std::num::NonZeroU64::new(
                                std::mem::size_of::<ColorAdjustParams>() as u64
                            )
                            .unwrap(),
                        ),
                    }),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Color Adjust Pass"),
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

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..6, 0..1);
    }

    fn reset_frame(&mut self) {
        self.frame_offset = 0;
    }
}
