use std::borrow::Cow;
use std::future::Future;
use std::task::{Context, Poll, Waker};

use bytemuck::{Pod, Zeroable};
use moyu_core::base::{MVPMatrix, Rect};
use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::RenderCommandSender;
use wgpu::util::DeviceExt;
use wgpu::*;

use crate::nodes::{ShaderParam, ShaderParamType, ShaderSource as ShaderNodeSource};

const SHADER_PARAM_SLOT_COUNT: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ShaderRenderUniform {
    position: [f32; 2],
    size: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ShaderBuiltinsUniform {
    time: f32,
    time_delta: f32,
    progress: f32,
    effect_id: i32,
    frame: u32,
    channel_count: u32,
    _padding1: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ShaderParamsUniform {
    slots: [u32; SHADER_PARAM_SLOT_COUNT],
}

#[derive(Clone, Copy)]
pub struct ShaderPassBuiltins {
    pub time: f32,
    pub time_delta: f32,
    pub progress: f32,
    pub effect_id: i32,
    pub frame: u32,
    pub channel_count: u32,
}

pub struct ShaderPass {
    format: TextureFormat,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    dummy_view: TextureView,
}

fn wait_for_future<T>(device: &Device, future: impl Future<Output = T>) -> T {
    let waker = Waker::noop();
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(waker);

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => {
                let _ = device.poll(wgpu::PollType::wait_indefinitely());
            }
        }
    }
}

fn pack_params(params: &[ShaderParam]) -> Result<ShaderParamsUniform, String> {
    let mut uniform = ShaderParamsUniform::zeroed();

    for (index, param) in params.iter().enumerate() {
        if index >= SHADER_PARAM_SLOT_COUNT {
            return Err(format!(
                "shader params exceed the current limit of {} 4-byte slots",
                SHADER_PARAM_SLOT_COUNT
            ));
        }

        match param.param_type {
            ShaderParamType::Float => {
                uniform.slots[index] = (param.value as f32).to_bits();
            }
            ShaderParamType::Int => {
                if param.value.fract() != 0.0 {
                    return Err(format!(
                        "shader int param '{}' must be an integer, got {}",
                        param.name, param.value
                    ));
                }

                if !(i32::MIN as f64..=i32::MAX as f64).contains(&param.value) {
                    return Err(format!(
                        "shader int param '{}' is out of i32 range: {}",
                        param.name, param.value
                    ));
                }

                uniform.slots[index] = param.value as i32 as u32;
            }
        }
    }

    Ok(uniform)
}

impl ShaderPass {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Shader Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
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
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 7,
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

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Shader Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let dummy_texture = device.create_texture(&TextureDescriptor {
            label: Some("Shader Dummy Texture"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: config.format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let dummy_view = dummy_texture.create_view(&TextureViewDescriptor::default());

        Self {
            format: config.format,
            bind_group_layout,
            sampler,
            dummy_view,
        }
    }

    pub fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn sampler(&self) -> &Sampler {
        &self.sampler
    }

    pub fn create_texture_view(
        &self,
        device: &Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> TextureView {
        device
            .create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: self.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&TextureViewDescriptor::default())
    }

    pub fn compile_pipeline(
        &self,
        device: &Device,
        shader_source: &ShaderNodeSource,
    ) -> Result<RenderPipeline, String> {
        let vertex_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader Vertex Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader_vertex.wgsl").into()),
        });

        let fragment_source = match shader_source {
            ShaderNodeSource::Builtin { .. } => {
                Cow::Borrowed(include_str!("shaders/shader_transition_builtin.wgsl"))
            }
            ShaderNodeSource::Raw { content } => Cow::Owned(content.clone()),
        };

        device.push_error_scope(wgpu::ErrorFilter::Validation);

        let fragment_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader Fragment Module"),
            source: wgpu::ShaderSource::Wgsl(fragment_source),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Shader Pipeline Layout"),
            bind_group_layouts: &[
                &MVPMatrix::bind_group_layout(device),
                &self.bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Shader Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vertex_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &fragment_module,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: self.format,
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

        if let Some(err) = wait_for_future(device, device.pop_error_scope()) {
            return Err(err.to_string());
        }

        Ok(pipeline)
    }

    pub fn ensure_uniform_buffers(
        &self,
        device: &Device,
        render_uniform_buffer: &mut Option<Buffer>,
        builtins_uniform_buffer: &mut Option<Buffer>,
        params_uniform_buffer: &mut Option<Buffer>,
    ) {
        if render_uniform_buffer.is_none() {
            *render_uniform_buffer = Some(device.create_buffer_init(&util::BufferInitDescriptor {
                label: Some("Shader Render Uniform Buffer"),
                contents: bytemuck::bytes_of(&ShaderRenderUniform::zeroed()),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            }));
        }

        if builtins_uniform_buffer.is_none() {
            *builtins_uniform_buffer =
                Some(device.create_buffer_init(&util::BufferInitDescriptor {
                    label: Some("Shader Builtins Uniform Buffer"),
                    contents: bytemuck::bytes_of(&ShaderBuiltinsUniform::zeroed()),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                }));
        }

        if params_uniform_buffer.is_none() {
            *params_uniform_buffer = Some(device.create_buffer_init(&util::BufferInitDescriptor {
                label: Some("Shader Params Uniform Buffer"),
                contents: bytemuck::bytes_of(&ShaderParamsUniform::zeroed()),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            }));
        }
    }

    pub fn create_bind_group(
        &self,
        device: &Device,
        render_uniform_buffer: &Buffer,
        builtins_uniform_buffer: &Buffer,
        params_uniform_buffer: &Buffer,
        channel_views: &[Option<TextureView>; 4],
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Shader Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: render_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: builtins_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: params_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(
                        channel_views[0].as_ref().unwrap_or(&self.dummy_view),
                    ),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(
                        channel_views[1].as_ref().unwrap_or(&self.dummy_view),
                    ),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(
                        channel_views[2].as_ref().unwrap_or(&self.dummy_view),
                    ),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(
                        channel_views[3].as_ref().unwrap_or(&self.dummy_view),
                    ),
                },
            ],
        })
    }

    pub fn write_render_uniform(
        &self,
        render_queue: &RenderCommandSender,
        buffer: &Buffer,
        rect: Rect,
    ) {
        let uniform = ShaderRenderUniform {
            position: [rect.x(), rect.y()],
            size: [rect.width(), rect.height()],
        };

        render_queue
            .send(RenderCommand::WriteBuffer {
                buffer: buffer.clone(),
                offset: 0,
                data: bytemuck::bytes_of(&uniform).to_vec(),
                use_staging_belt: true,
            })
            .unwrap();
    }

    pub fn write_builtins_uniform(
        &self,
        render_queue: &RenderCommandSender,
        buffer: &Buffer,
        builtins: ShaderPassBuiltins,
    ) {
        let uniform = ShaderBuiltinsUniform {
            time: builtins.time,
            time_delta: builtins.time_delta,
            progress: builtins.progress,
            effect_id: builtins.effect_id,
            frame: builtins.frame,
            channel_count: builtins.channel_count,
            _padding1: [0; 2],
        };

        render_queue
            .send(RenderCommand::WriteBuffer {
                buffer: buffer.clone(),
                offset: 0,
                data: bytemuck::bytes_of(&uniform).to_vec(),
                use_staging_belt: true,
            })
            .unwrap();
    }

    pub fn write_params_uniform(
        &self,
        render_queue: &RenderCommandSender,
        buffer: &Buffer,
        params: &[ShaderParam],
    ) -> Result<(), String> {
        let uniform = pack_params(params)?;

        render_queue
            .send(RenderCommand::WriteBuffer {
                buffer: buffer.clone(),
                offset: 0,
                data: bytemuck::bytes_of(&uniform).to_vec(),
                use_staging_belt: true,
            })
            .unwrap();

        Ok(())
    }
}
