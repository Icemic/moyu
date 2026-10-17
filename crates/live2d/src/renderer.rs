//! wgpu rendering of a live2d model from the frame snapshot.
//!
//! One node draws in three steps: mask passes fill one texture per mask buffer,
//! then every drawable draws once with its mask coefficient sampled from it.
//! Everything in the snapshot is in canvas pixel space; the vertex shaders
//! compose it with the node transform and the engine MVP.
//!
//! Mask textures are sized by the plugin while the engine's offscreen passes
//! always cover the whole render surface, so the mask pass scales the plugin's
//! clip space onto the texture's share of the surface (see [`mask_correction`])
//! and covers the whole texture with the "not covered" value before the shapes
//! write. That assumes the mask texture is not larger than the surface, which
//! holds for the sizes the plugin asks for.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use moyu_core::base::{MVPMatrix, Rect, VertexDesc};
use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::{Node, NodeBaseTrait, RenderCommandSender, Renderer, RendererUpdatePayload};
use moyu_live2d_abi::Matrix4;
use moyu_live2d_abi::frame::{
    BLEND_ADD, BLEND_MULTIPLY, FLAG_DOUBLE_SIDED, FLAG_MASK_INVERTED, FLAG_VISIBLE,
};
use moyu_resource::types::{Asset, AssetId, AssetKind, TextureStatus};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::*;

use crate::event::Live2dEvent;
use crate::frame::{DrawableSnapshot, FrameSnapshot};
use crate::job;
use crate::node::Live2dNode;

/// Mask textures are single-sample RGBA: one channel per mask group.
const MASK_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

/// Uniform of the drawing pass, one per drawable.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct DrawableUniform {
    /// Canvas pixels to stage units; row-major, applied as `v * node`.
    node: [[f32; 4]; 4],
    /// Canvas pixels to mask sampling UV; row-major, applied as `v * clip`.
    clip: [[f32; 4]; 4],
    multiply: [f32; 4],
    screen: [f32; 4],
    /// `rgb` is the node tint, `a` the combined opacity.
    base: [f32; 4],
    /// One-hot mask channel.
    channel: [f32; 4],
    /// `x`: 0 no mask, 1 normal, 2 inverted.
    mask_mode: [f32; 4],
}

/// Uniform of the mask pass, one per mask command.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct MaskUniform {
    /// Canvas pixels to mask clip space; row-major, applied as `v * matrix`.
    matrix: [[f32; 4]; 4],
    /// `[x_min, y_min, x_max, y_max]` in that clip space.
    clip_rect: [f32; 4],
    /// One-hot channel this command writes.
    channel: [f32; 4],
}

/// Vertex of the full-buffer fill; positions are already in clip space.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct FillVertex {
    position: [f32; 2],
}

impl VertexDesc for FillVertex {
    fn attribs() -> &'static [VertexAttribute] {
        static ATTRIBS: [VertexAttribute; 1] = vertex_attr_array![0 => Float32x2];
        &ATTRIBS
    }

    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: Self::attribs(),
        }
    }
}

/// Drawable vertex: canvas pixel position and normalized UV.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Live2dVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl VertexDesc for Live2dVertex {
    fn attribs() -> &'static [VertexAttribute] {
        static ATTRIBS: [VertexAttribute; 2] =
            vertex_attr_array![0 => Float32x2, 1 => Float32x2];
        &ATTRIBS
    }

    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: Self::attribs(),
        }
    }
}

/// Blend bucket of a drawable: 0 normal, 1 add, 2 multiply.
///
/// The plugin already picked the effective mode, so this only maps the contract
/// value onto the pipeline set.
fn blend_bucket(drawable: &DrawableSnapshot) -> usize {
    match drawable.blend_mode {
        BLEND_ADD => 1,
        BLEND_MULTIPLY => 2,
        _ => 0,
    }
}

/// GPU resources of one drawable.
struct DrawableGpu {
    uniform_buffer: Buffer,
    uniform_group: BindGroup,
    vertex_buffer: Buffer,
    vertex_capacity: u64,
    index_buffer: Buffer,
    index_capacity: u64,
}

/// GPU resources of one mask buffer and its commands.
struct MaskGpu {
    /// Kept for the texture's lifetime; the view is what draws use.
    _texture: Texture,
    view: TextureView,
    width: u32,
    height: u32,
    /// Uniform buffers per command, reused across frames.
    commands: Vec<MaskCommandGpu>,
    /// Commands of the frame being built.
    used: usize,
}

/// GPU resources of one mask command.
struct MaskCommandGpu {
    uniform_buffer: Buffer,
    uniform_group: BindGroup,
}

/// One drawable draw of the current plan.
struct DrawItem {
    /// Slot in the GPU drawable list, which mirrors the snapshot order.
    slot: usize,
    /// Index into the texture list of the model.
    texture: usize,
    /// Blend bucket, see [`blend_bucket`].
    bucket: usize,
    /// Whether the drawable draws both sides.
    double_sided: bool,
    count: u32,
    /// Mask buffer the drawable samples, when it is masked.
    mask: Option<usize>,
}

/// One shape inside a mask pass.
struct MaskDrawItem {
    /// Position of the command's uniform inside the mask buffer.
    uniform: usize,
    /// Slot of the drawable whose shape draws the mask.
    slot: usize,
    /// Index into the texture list of the model.
    texture: usize,
    count: u32,
}

/// One mask pass: everything one mask buffer needs before the draws sample it.
struct MaskPass {
    /// Index into the GPU mask list.
    mask: usize,
    draws: Vec<MaskDrawItem>,
}

/// Everything `collect_commands` needs, rebuilt on every renderer update.
#[derive(Default)]
struct FramePlan {
    mask_passes: Vec<MaskPass>,
    draws: Vec<DrawItem>,
    /// Pass origin in stage units that puts an offscreen viewport at the
    /// target's top left corner.
    mask_origin: (f32, f32),
    /// Whether a frame has been uploaded.
    has_frame: bool,
    /// Whether that frame is complete, that is every texture is ready.
    complete: bool,
}

/// GPU state of one node, created by the renderer.
pub struct Live2dGpu {
    /// Model epoch the resources belong to.
    epoch: u64,
    /// Whether the node has drawn a complete frame.
    pub ready: bool,
    drawables: Vec<DrawableGpu>,
    masks: Vec<MaskGpu>,
    /// Bind groups of the mask textures, recreated with them.
    mask_groups: Vec<BindGroup>,
    /// Loaded textures of the model, indexed like the plugin's texture list.
    texture_ids: Vec<Option<Arc<AssetId>>>,
    /// Texture paths the loaded assets were requested for.
    texture_paths: Vec<String>,
    /// Bind groups of the loaded textures, indexed like the texture list.
    texture_groups: Vec<Option<BindGroup>>,
    /// Mask size of the stored frame; a change recreates the textures.
    mask_size: (u32, u32),
    /// The stored frame: its geometry lives in the buffers and the plan is
    /// rebuilt from it until a newer frame arrives.
    frame: Option<FrameSnapshot>,
    plan: FramePlan,
    /// Whether a texture failure was already reported.
    texture_error: bool,
}

impl std::fmt::Debug for Live2dGpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Live2dGpu")
            .field("epoch", &self.epoch)
            .field("drawables", &self.drawables.len())
            .field("masks", &self.masks.len())
            .finish_non_exhaustive()
    }
}

impl Live2dGpu {
    fn new(epoch: u64) -> Self {
        Self {
            epoch,
            ready: false,
            drawables: Vec::new(),
            masks: Vec::new(),
            mask_groups: Vec::new(),
            texture_ids: Vec::new(),
            texture_paths: Vec::new(),
            texture_groups: Vec::new(),
            mask_size: (0, 0),
            frame: None,
            plan: FramePlan::default(),
            texture_error: false,
        }
    }
}

/// The renderer: pipelines and samplers shared by every live2d node.
pub struct Live2dRenderer {
    draw_pipelines: [[RenderPipeline; 2]; 3],
    mask_pipeline: RenderPipeline,
    fill_pipeline: RenderPipeline,
    drawable_layout: BindGroupLayout,
    mask_layout: BindGroupLayout,
    texture_layout: BindGroupLayout,
    sampler: Sampler,
    fill_buffer: Buffer,
    fill_group: BindGroup,
}

impl Live2dRenderer {
    pub fn new(device: &Device, format: TextureFormat, sample_count: u32) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Live2D Shader"),
            source: ShaderSource::Wgsl(include_str!("./shaders/live2d.wgsl").into()),
        });

        let drawable_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Live2D Drawable Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(
                        std::mem::size_of::<DrawableUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

        let mask_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Live2D Mask Layout"),
            entries: &[BindGroupLayoutEntry {
                // Binding 0 is the drawable uniform, so the mask uniform sits on
                // the next slot; the mask pipelines' layouts only carry this one.
                binding: 1,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(std::mem::size_of::<MaskUniform>() as u64),
                },
                count: None,
            }],
        });

        let texture_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Live2D Texture Layout"),
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
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Live2D Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });

        let mvp_layout = MVPMatrix::bind_group_layout(device);

        // The drawing pipelines: three blend buckets (0 normal, 1 add, 2
        // multiply), each with and without face culling. The fragment shader
        // emits premultiplied alpha, so rgb and alpha blend separately.
        let create_draw = |label: &str, blend: usize, cull_double_sided: bool| {
            let blend_state = BlendState {
                color: match blend {
                    1 => BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    2 => BlendComponent {
                        src_factor: BlendFactor::Dst,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                    _ => BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                },
                alpha: match blend {
                    0 => BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                    _ => BlendComponent {
                        src_factor: BlendFactor::Zero,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                },
            };

            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[
                    Some(&mvp_layout),
                    Some(&drawable_layout),
                    Some(&texture_layout),
                    Some(&texture_layout),
                ],
                immediate_size: 0,
            });

            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_draw"),
                    compilation_options: Default::default(),
                    buffers: &[Some(Live2dVertex::desc())],
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: Some("fs_draw"),
                    compilation_options: Default::default(),
                    targets: &[Some(ColorTargetState {
                        format,
                        blend: Some(blend_state),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    // Two y flips happen on the way from model space to clip
                    // space, so the official counter-clockwise winding holds.
                    front_face: FrontFace::Ccw,
                    cull_mode: if cull_double_sided {
                        None
                    } else {
                        Some(Face::Back)
                    },
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: sample_count,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            })
        };

        let draw_pipelines = [
            [
                create_draw("Live2D Draw Normal Culled", 0, false),
                create_draw("Live2D Draw Normal", 0, true),
            ],
            [
                create_draw("Live2D Draw Add Culled", 1, false),
                create_draw("Live2D Draw Add", 1, true),
            ],
            [
                create_draw("Live2D Draw Multiply Culled", 2, false),
                create_draw("Live2D Draw Multiply", 2, true),
            ],
        ];

        // The mask pass is single-sample into our own RGBA texture.
        let mask_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Live2D Mask Pipeline Layout"),
            bind_group_layouts: &[Some(&mvp_layout), Some(&mask_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let mask_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Live2D Mask Pipeline"),
            layout: Some(&mask_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_mask"),
                compilation_options: Default::default(),
                buffers: &[Some(Live2dVertex::desc())],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_mask"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: MASK_FORMAT,
                    blend: Some(BlendState {
                        // The mask accumulates `dst = dst * (1 - src)` per channel.
                        color: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::OneMinusSrc,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                // Mask geometry draws both sides, like the official backends.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        // The clear replacement: a full-buffer quad writing white, so the mask
        // starts at "nothing covered" independent of the pass load op.
        let fill_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Live2D Mask Fill Layout"),
            bind_group_layouts: &[Some(&mvp_layout), Some(&mask_layout)],
            immediate_size: 0,
        });
        let fill_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Live2D Mask Fill Pipeline"),
            layout: Some(&fill_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_fill"),
                compilation_options: Default::default(),
                buffers: &[Some(FillVertex::desc())],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_fill"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: MASK_FORMAT,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let fill_vertices = [
            FillVertex {
                position: [-1.0, -1.0],
            },
            FillVertex { position: [1.0, -1.0] },
            FillVertex { position: [1.0, 1.0] },
            FillVertex {
                position: [-1.0, -1.0],
            },
            FillVertex { position: [1.0, 1.0] },
            FillVertex { position: [-1.0, 1.0] },
        ];
        let fill_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Live2D Mask Fill Quad"),
            contents: bytemuck::cast_slice(&fill_vertices),
            usage: BufferUsages::VERTEX,
        });

        // A 1x1 mask texture keeps the binding valid for unmasked draws. Its
        // contents are never sampled (`mask_mode` skips the branch), so it stays
        // uninitialized.
        let placeholder = device.create_texture(&TextureDescriptor {
            label: Some("Live2D Placeholder Mask"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: MASK_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let placeholder_view = placeholder.create_view(&TextureViewDescriptor::default());

        let fill_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Live2D Placeholder Mask Group"),
            layout: &texture_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&placeholder_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            draw_pipelines,
            mask_pipeline,
            fill_pipeline,
            drawable_layout,
            mask_layout,
            texture_layout,
            sampler,
            fill_buffer,
            fill_group,
        }
    }

    /// Creates the group that binds one texture.
    fn texture_group(&self, device: &Device, view: &TextureView) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Live2D Texture Group"),
            layout: &self.texture_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Creates a mask texture of the given size.
    fn create_mask_texture(&self, device: &Device, width: u32, height: u32) -> (Texture, TextureView) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Live2D Mask Texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: MASK_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }

    /// Requests the model's textures once the driver reports its texture list.
    ///
    /// Both only change when the model does, so this is a no-op afterwards.
    fn request_textures(
        &self,
        gpu: &mut Live2dGpu,
        info: &job::ModelInfo,
        payload: &RendererUpdatePayload,
    ) {
        if gpu.texture_paths == info.textures {
            return;
        }
        gpu.texture_paths = info.textures.clone();
        gpu.texture_ids = info
            .textures
            .iter()
            .map(|path| Some(payload.resource_manager.load_asset(AssetKind::Texture, path)))
            .collect();
        gpu.texture_groups = vec![None; info.textures.len()];
        gpu.texture_error = false;
    }

    /// Builds the bind groups of the loaded textures and reports whether any of
    /// them is still missing.
    fn resolve_textures(&self, device: &Device, gpu: &mut Live2dGpu, node_id: u32) -> bool {
        let mut pending = false;
        let mut views: Vec<Option<Arc<TextureView>>> = Vec::with_capacity(gpu.texture_ids.len());
        for (index, id) in gpu.texture_ids.iter().enumerate() {
            let path = gpu.texture_paths.get(index).map(String::as_str).unwrap_or("");
            let Some(id) = id else {
                views.push(None);
                pending = true;
                continue;
            };
            let asset = id.asset_unchecked();
            let Asset::Texture(texture) = asset.as_ref() else {
                views.push(None);
                pending = true;
                continue;
            };
            match texture.status() {
                TextureStatus::Ready => match texture.view.load_full() {
                    Some(view) => views.push(Some(view)),
                    None => {
                        views.push(None);
                        pending = true;
                    }
                },
                // A texture that failed to load never becomes ready, so say so
                // once instead of waiting forever.
                TextureStatus::Error => {
                    views.push(None);
                    pending = true;
                    if !gpu.texture_error {
                        gpu.texture_error = true;
                        job::send_event(
                            node_id,
                            Live2dEvent::Error {
                                code: moyu_live2d_abi::error::FILE,
                                message: format!("the live2d texture failed to load: {path}"),
                            },
                        );
                    }
                }
                _ => {
                    views.push(None);
                    pending = true;
                }
            }
        }

        if gpu.texture_groups.len() != views.len() {
            gpu.texture_groups = vec![None; views.len()];
        }
        for (index, view) in views.into_iter().enumerate() {
            if gpu.texture_groups[index].is_none()
                && let Some(view) = view
            {
                gpu.texture_groups[index] = Some(self.texture_group(device, &view));
            }
        }
        pending
    }

    /// Uploads the stored frame's geometry into the per-drawable buffers.
    ///
    /// The gpu-upload bytes are moved into the render commands, so the stored
    /// frame keeps only its metadata afterwards.
    fn upload_geometry(
        &self,
        device: &Device,
        render_queue: &RenderCommandSender,
        gpu: &mut Live2dGpu,
    ) {
        let Some(frame) = gpu.frame.as_mut() else {
            return;
        };

        // One slot per drawable, in snapshot order.
        while gpu.drawables.len() < frame.drawables.len() {
            let uniform_buffer = device.create_buffer(&BufferDescriptor {
                label: Some("Live2D Drawable Uniform"),
                size: std::mem::size_of::<DrawableUniform>() as u64,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let uniform_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("Live2D Drawable Uniform Group"),
                layout: &self.drawable_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });
            gpu.drawables.push(DrawableGpu {
                uniform_buffer,
                uniform_group,
                vertex_buffer: device.create_buffer(&BufferDescriptor {
                    label: Some("Live2D Vertex Buffer"),
                    size: 1024,
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                vertex_capacity: 1024,
                index_buffer: device.create_buffer(&BufferDescriptor {
                    label: Some("Live2D Index Buffer"),
                    size: 512,
                    usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                index_capacity: 512,
            });
        }

        for (index, drawable) in frame.drawables.iter_mut().enumerate() {
            let slot = &mut gpu.drawables[index];

            let vertex_data = std::mem::take(&mut drawable.vertex_data);
            if !vertex_data.is_empty() {
                if vertex_data.len() as u64 > slot.vertex_capacity {
                    let capacity = (vertex_data.len() as u64).next_power_of_two().max(1024);
                    slot.vertex_buffer = device.create_buffer(&BufferDescriptor {
                        label: Some("Live2D Vertex Buffer"),
                        size: capacity,
                        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    slot.vertex_capacity = capacity;
                }
                render_queue
                    .send(RenderCommand::WriteBuffer {
                        buffer: slot.vertex_buffer.clone(),
                        offset: 0,
                        data: vertex_data,
                        use_staging_belt: true,
                    })
                    .ok();
            }

            let index_data = std::mem::take(&mut drawable.index_data);
            if !index_data.is_empty() {
                // Drawables are drawn in render order and models may animate that
                // order, so a slot's indices can change between frames: they go
                // up with every frame.
                if index_data.len() as u64 > slot.index_capacity {
                    let capacity = (index_data.len() as u64).next_power_of_two().max(512);
                    slot.index_buffer = device.create_buffer(&BufferDescriptor {
                        label: Some("Live2D Index Buffer"),
                        size: capacity,
                        usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    slot.index_capacity = capacity;
                }
                render_queue
                    .send(RenderCommand::WriteBuffer {
                        buffer: slot.index_buffer.clone(),
                        offset: 0,
                        data: index_data,
                        use_staging_belt: true,
                    })
                    .ok();
            }
        }
    }

    /// Creates or recreates the mask textures of the stored frame.
    fn sync_masks(&self, device: &Device, gpu: &mut Live2dGpu) {
        let Some(frame) = gpu.frame.as_ref() else {
            return;
        };
        let size = frame.mask_buffers.first().copied().unwrap_or((0, 0));
        if size == gpu.mask_size && gpu.masks.len() == frame.mask_buffers.len() {
            return;
        }

        gpu.mask_size = size;
        gpu.masks = frame
            .mask_buffers
            .iter()
            .map(|(width, height)| {
                let (texture, view) = self.create_mask_texture(device, *width, *height);
                MaskGpu {
                    _texture: texture,
                    view,
                    width: *width,
                    height: *height,
                    commands: Vec::new(),
                    used: 0,
                }
            })
            .collect();
        // Draws bind the mask texture through these groups, so they follow every
        // recreation.
        gpu.mask_groups = gpu
            .masks
            .iter()
            .map(|mask| self.texture_group(device, &mask.view))
            .collect();
    }
}

/// Builds the uniform of one drawable.
///
/// The node values are passed in because the GPU state already borrows the node
/// mutably at the call site.
fn drawable_uniform(
    node_matrix: [[f32; 4]; 4],
    tint: [f32; 4],
    global_opacity: f32,
    drawable: &DrawableSnapshot,
) -> DrawableUniform {
    let opacity = drawable.opacity * global_opacity;
    let mut channel = [0.0f32; 4];
    let mask_mode = if drawable.mask_buffer_index >= 0 {
        channel[drawable.mask_channel.min(3) as usize] = 1.0;
        if drawable.flags & FLAG_MASK_INVERTED != 0 {
            2.0
        } else {
            1.0
        }
    } else {
        0.0
    };

    DrawableUniform {
        node: node_matrix,
        clip: drawable.clip_matrix,
        multiply: [
            drawable.multiply[0],
            drawable.multiply[1],
            drawable.multiply[2],
            1.0,
        ],
        screen: [
            drawable.screen[0],
            drawable.screen[1],
            drawable.screen[2],
            1.0,
        ],
        base: [tint[0], tint[1], tint[2], opacity],
        channel,
        mask_mode: [mask_mode, 0.0, 0.0, 0.0],
    }
}

/// Scale and offset that map the plugin's mask clip space onto the mask
/// texture's share of the render target: `x' = scale * x + offset`.
///
/// The plugin lays a mask buffer out in its own `width x height` space and hands
/// out clip coordinates covering exactly that space, but the engine gives an
/// offscreen pass a viewport as large as the whole render surface. Scaling the
/// plugin's clip space by `texture / surface` puts the mask back on exactly the
/// texture's pixels instead of stretching it over the surface.
fn mask_correction(mask: (u32, u32), surface: (f32, f32)) -> [f32; 4] {
    let scale_x = mask.0 as f32 / surface.0.max(1.0);
    let scale_y = mask.1 as f32 / surface.1.max(1.0);
    [scale_x, scale_y, scale_x - 1.0, 1.0 - scale_y]
}

/// Applies a mask correction to a canvas-to-mask-clip transform.
///
/// Both matrices are row-major with the row-vector convention, so the correction
/// composes on the right: `v * matrix * correction`. With `correction` nonzero
/// only on its diagonal and translation row, the product reduces to a scale of
/// the first two columns plus a translation along the fourth.
fn corrected_matrix(matrix: &Matrix4, correction: [f32; 4]) -> [[f32; 4]; 4] {
    let [scale_x, scale_y, offset_x, offset_y] = correction;
    let mut out = *matrix;
    for row in &mut out {
        row[0] = row[0] * scale_x + row[3] * offset_x;
        row[1] = row[1] * scale_y + row[3] * offset_y;
    }
    out
}

/// Applies a mask correction to a clip space rectangle.
///
/// Both scales are positive, so the corner order of the rectangle survives.
fn corrected_rect(rect: [f32; 4], correction: [f32; 4]) -> [f32; 4] {
    let [scale_x, scale_y, offset_x, offset_y] = correction;
    [
        scale_x * rect[0] + offset_x,
        scale_y * rect[1] + offset_y,
        scale_x * rect[2] + offset_x,
        scale_y * rect[3] + offset_y,
    ]
}

/// Converts the node transform into a row-major matrix for the shader.
///
/// The engine stores transforms column-major and applies them as `matrix * v`,
/// while the snapshot matrices reach the shader row-major and applied as
/// `v * matrix`; the transpose lines the node up with the latter.
fn glam_mat4(transform: &moyu_core::base::Transform) -> [[f32; 4]; 4] {
    let affine: glam::Affine3A = **transform;
    glam::Mat4::from(affine).transpose().to_cols_array_2d()
}

impl Renderer for Live2dRenderer {
    fn name(&self) -> &'static str {
        "live2d"
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        &self.draw_pipelines[0][0]
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.texture_layout
    }

    fn prepare(
        &mut self,
        node: &mut dyn Node,
        _device: &Device,
        _queue: &Queue,
        payload: &RendererUpdatePayload,
    ) {
        let node = node.as_any_mut().downcast_mut::<Live2dNode>().unwrap();
        let Some(info) = node.job.state.lock().model_info.clone() else {
            return;
        };

        // The canvas is the node's intrinsic size, and the textures are
        // requested here so they load while the script keeps running.
        if node.base().intrinsic_size() != (info.canvas_size[0], info.canvas_size[1]) {
            node.base_mut()
                .set_intrinsic_size(info.canvas_size[0], info.canvas_size[1]);
        }

        let gpu = node.gpu.get_or_insert_with(|| Box::new(Live2dGpu::new(info.epoch)));
        if gpu.epoch != info.epoch {
            **gpu = Live2dGpu::new(info.epoch);
        }
        self.request_textures(gpu, &info, payload);
    }

    fn update(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        _queue: &Queue,
        render_queue: &RenderCommandSender,
        payload: &RendererUpdatePayload,
    ) {
        let node = node.as_any_mut().downcast_mut::<Live2dNode>().unwrap();

        // The node values the uniforms need, read before the GPU state borrows
        // the node mutably.
        let node_id = *node.base().id();
        let node_matrix = glam_mat4(node.base().global_transform());
        let tint = node.base().tint().to_array();
        let global_opacity = *node.base().global_opacity();

        // The newest frame and the model it belongs to are read in one go, so the
        // renderer can never pair a frame with another model's resources.
        let (info, snapshot) = {
            let mut state = node.job.state.lock();
            (state.model_info.clone(), state.snapshot.take())
        };
        let epoch = info.as_ref().map_or(job::NO_MODEL_EPOCH, |info| info.epoch);
        if node.gpu.as_ref().is_none_or(|gpu| gpu.epoch != epoch) {
            node.gpu = Some(Box::new(Live2dGpu::new(epoch)));
        }
        let gpu = node.gpu.as_mut().unwrap();

        let Some(info) = info else {
            // Without a model there is nothing to draw.
            gpu.plan = FramePlan::default();
            gpu.ready = false;
            return;
        };
        self.request_textures(gpu, &info, payload);

        // A new frame replaces the stored one and uploads its geometry once; the
        // plan below redraws it until a newer frame arrives, because the engine
        // clears the frame target on every frame.
        if let Some(snapshot) = snapshot {
            gpu.frame = Some(snapshot);
            gpu.plan.has_frame = true;
            self.upload_geometry(device, render_queue, gpu);
        }
        self.sync_masks(device, gpu);

        // The plan is rebuilt from scratch on every frame.
        gpu.plan.mask_passes.clear();
        gpu.plan.draws.clear();
        gpu.plan.complete = false;
        gpu.ready = false;
        for mask in &mut gpu.masks {
            mask.used = 0;
        }

        // A frame that cannot get all of its textures yet would draw with gaps,
        // so the node keeps showing nothing until the model is fully loaded.
        let textures_pending = self.resolve_textures(device, gpu, node_id);
        if gpu.frame.is_none() || textures_pending {
            return;
        }

        // The engine's offscreen viewport starts at the pass origin, which the
        // stage transform can shift; the mask pass needs it at the target's top
        // left corner, so it asks for that origin instead of the default one.
        let (scale, translate_x, translate_y) = moyu_core::base::get_scale_and_translate(
            payload.stage_logical_size.0,
            payload.stage_logical_size.1,
            payload.surface_logical_size.0,
            payload.surface_logical_size.1,
        );
        gpu.plan.mask_origin = if scale > 0.0 {
            (-translate_x / scale, -translate_y / scale)
        } else {
            (0.0, 0.0)
        };
        let surface = (
            payload.surface_logical_size.0 * payload.scale_factor,
            payload.surface_logical_size.1 * payload.scale_factor,
        );

        let Some(frame) = gpu.frame.as_ref() else {
            return;
        };

        // Mask commands: their uniforms are rewritten on every frame through the
        // render queue, which keeps them ordered against the draws, and they are
        // grouped per mask buffer because the plugin only promises the execution
        // order of its commands, not an order per buffer.
        let mut mask_plan: Vec<Vec<MaskDrawItem>> =
            (0..gpu.masks.len()).map(|_| Vec::new()).collect();
        for command in &frame.mask_commands {
            let buffer = command.buffer_index as usize;
            let Some(mask) = gpu.masks.get_mut(buffer) else {
                continue;
            };
            let Some(drawable) = frame.drawables.get(command.drawable_index) else {
                continue;
            };
            let texture = drawable.texture_index.max(0) as usize;
            if gpu.texture_groups.get(texture).is_none_or(|group| group.is_none()) {
                continue;
            }

            // The plugin's mask space covers the mask texture; the pass draws
            // into the texture's share of the surface.
            let correction = mask_correction((mask.width, mask.height), surface);
            let uniform = MaskUniform {
                matrix: corrected_matrix(&command.matrix, correction),
                clip_rect: corrected_rect(command.clip_rect, correction),
                channel: one_hot(command.channel),
            };

            let position = mask.used;
            mask.used += 1;
            if position == mask.commands.len() {
                let uniform_buffer = device.create_buffer(&BufferDescriptor {
                    label: Some("Live2D Mask Uniform"),
                    size: std::mem::size_of::<MaskUniform>() as u64,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let uniform_group = device.create_bind_group(&BindGroupDescriptor {
                    label: Some("Live2D Mask Uniform Group"),
                    layout: &self.mask_layout,
                    entries: &[BindGroupEntry {
                        binding: 1,
                        resource: uniform_buffer.as_entire_binding(),
                    }],
                });
                mask.commands.push(MaskCommandGpu {
                    uniform_buffer,
                    uniform_group,
                });
            }
            render_queue
                .send(RenderCommand::WriteBuffer {
                    buffer: mask.commands[position].uniform_buffer.clone(),
                    offset: 0,
                    data: bytemuck::bytes_of(&uniform).to_vec(),
                    use_staging_belt: true,
                })
                .ok();
            mask_plan[buffer].push(MaskDrawItem {
                uniform: position,
                slot: command.drawable_index,
                texture,
                count: drawable.index_count as u32,
            });
        }

        // Drawables in render order; the snapshot already sorts them.
        for (slot, drawable) in frame.drawables.iter().enumerate() {
            if drawable.flags & FLAG_VISIBLE == 0 || drawable.index_count == 0 {
                continue;
            }
            let texture = drawable.texture_index.max(0) as usize;
            if gpu.texture_groups.get(texture).is_none_or(|group| group.is_none()) {
                continue;
            }
            let Some(gpu_slot) = gpu.drawables.get(slot) else {
                continue;
            };

            // Uniforms change with every frame (node transform, tint, opacity,
            // mask layout) and a plain queue write is not tied to this frame's
            // submission, so they go through the render queue as well.
            let uniform = drawable_uniform(node_matrix, tint, global_opacity, drawable);
            render_queue
                .send(RenderCommand::WriteBuffer {
                    buffer: gpu_slot.uniform_buffer.clone(),
                    offset: 0,
                    data: bytemuck::bytes_of(&uniform).to_vec(),
                    use_staging_belt: true,
                })
                .ok();

            let mask = (drawable.mask_buffer_index >= 0)
                .then_some(drawable.mask_buffer_index as usize)
                .filter(|index| *index < gpu.masks.len());
            gpu.plan.draws.push(DrawItem {
                slot,
                texture,
                bucket: blend_bucket(drawable),
                double_sided: drawable.flags & FLAG_DOUBLE_SIDED != 0,
                count: drawable.index_count as u32,
                mask,
            });
        }

        for (mask, draws) in mask_plan.into_iter().enumerate() {
            if draws.is_empty() {
                continue;
            }
            gpu.plan.mask_passes.push(MaskPass { mask, draws });
        }

        gpu.plan.complete = true;
        gpu.ready = true;
    }

    fn collect_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        let Some(node) = node.as_any().downcast_ref::<Live2dNode>() else {
            return;
        };
        let Some(gpu) = node.gpu.as_deref() else {
            return;
        };
        if !gpu.plan.complete {
            return;
        }

        // Mask passes first: they fill the textures the draws sample. The fill
        // covers the whole texture, which holds because a mask texture is never
        // larger than the render surface. The contract fixes the clearing point:
        // every buffer starts the frame filled with 1.0, before any of its
        // commands run, so one fill per pass is all a buffer needs.
        for pass in &gpu.plan.mask_passes {
            let mask = &gpu.masks[pass.mask];
            let Some(first) = pass.draws.first() else {
                continue;
            };

            render_queue
                .send(RenderCommand::BeginRenderTargetPass {
                    target_view: mask.view.clone(),
                    resolve_view: None,
                    rect: Rect::new(0.0, 0.0, 0.0, 0.0),
                    content_origin: Some(gpu.plan.mask_origin),
                })
                .ok();

            // White is "not covered" in every mask channel, so the pass starts
            // from a filled texture instead of the engine's transparent clear.
            render_queue
                .send(RenderCommand::Draw {
                    pipeline: self.fill_pipeline.clone(),
                    bind_group: mask.commands[first.uniform].uniform_group.clone(),
                    extra_bind_groups: Vec::new(),
                    vertex_buffer: Some(self.fill_buffer.clone()),
                    index_buffer: None,
                    instance_buffer: None,
                    count: 6,
                    instance_count: 1,
                })
                .ok();

            for item in &pass.draws {
                let Some(slot) = gpu.drawables.get(item.slot) else {
                    continue;
                };
                let Some(command) = mask.commands.get(item.uniform) else {
                    continue;
                };
                let Some(texture) = gpu
                    .texture_groups
                    .get(item.texture)
                    .and_then(|group| group.as_ref())
                else {
                    continue;
                };
                render_queue
                    .send(RenderCommand::Draw {
                        pipeline: self.mask_pipeline.clone(),
                        bind_group: command.uniform_group.clone(),
                        extra_bind_groups: vec![texture.clone()],
                        vertex_buffer: Some(slot.vertex_buffer.clone()),
                        index_buffer: Some(slot.index_buffer.clone()),
                        instance_buffer: None,
                        count: item.count,
                        instance_count: 1,
                    })
                    .ok();
            }

            render_queue.send(RenderCommand::EndRenderTargetPass).ok();
        }

        for item in &gpu.plan.draws {
            let Some(slot) = gpu.drawables.get(item.slot) else {
                continue;
            };
            let Some(texture) = gpu
                .texture_groups
                .get(item.texture)
                .and_then(|group| group.as_ref())
            else {
                continue;
            };
            let mask_group = item
                .mask
                .and_then(|index| gpu.mask_groups.get(index))
                .unwrap_or(&self.fill_group);

            render_queue
                .send(RenderCommand::Draw {
                    pipeline: self.draw_pipelines[item.bucket][usize::from(item.double_sided)]
                        .clone(),
                    bind_group: slot.uniform_group.clone(),
                    extra_bind_groups: vec![texture.clone(), mask_group.clone()],
                    vertex_buffer: Some(slot.vertex_buffer.clone()),
                    index_buffer: Some(slot.index_buffer.clone()),
                    instance_buffer: None,
                    count: item.count,
                    instance_count: 1,
                })
                .ok();
        }
    }
}

/// Builds the one-hot channel vector of a mask command.
fn one_hot(channel: u32) -> [f32; 4] {
    let mut out = [0.0f32; 4];
    out[channel.min(3) as usize] = 1.0;
    out
}
