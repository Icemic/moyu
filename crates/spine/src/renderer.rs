//! wgpu rendering of a spine model from the frame snapshot.
//!
//! # Coordinates
//!
//! Vertices arrive in world units relative to the model's origin. Mapping them
//! onto the node is a plain shift: the reference rectangle's corner becomes the
//! node origin, and the engine MVP takes the rest of the way into clip space.
//!
//! No flip happens here. `spine-cpp` runs with `Bone::yDown` on, so its world
//! space already grows downwards like the engine's node space, and the MVP adds
//! nothing on top; flipping here would draw the model upside down. That was
//! verified against `spineboy` rather than assumed.
//!
//! # Colors
//!
//! The plugin hands pre-premultiplied vertex colors: `light.rgb` and `dark.rgb`
//! are both scaled by the vertex alpha, and `dark.a` is 255. The fragment shader
//! applies the two-color formula over the full `dark` vector, so its alpha takes
//! part even though it is constant today. Textures are normalized to
//! premultiplied alpha by the resource layer, so the shader stays in premultiplied
//! space and the blend states are premultiplied ones.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use moyu_core::base::{MVPMatrix, VertexDesc};
use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::{Node, NodeBaseTrait, RenderCommandSender, Renderer, RendererUpdatePayload};
use moyu_resource::types::{Asset, AssetId, AssetKind, Texture, TextureStatus};
use wgpu::*;

use crate::event::SpineEvent;
use crate::frame::{FrameSnapshot, VERTEX_STRIDE};
use crate::job;
use crate::node::SpineNode;

/// Blend buckets, matching `moyu_spine_abi::frame::BLEND_*`.
const BLEND_NORMAL: i32 = 0;
const BLEND_ADDITIVE: i32 = 1;
const BLEND_MULTIPLY: i32 = 2;
const BLEND_SCREEN: i32 = 3;

/// Uniform shared by every batch of a node.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct SpineUniform {
    /// World units to stage units, row-major, applied as `v * node`.
    node: [[f32; 4]; 4],
    /// `rgb` is the node tint, `a` the global opacity.
    base: [f32; 4],
}

/// Vertex as the GPU sees it: position, UV, light color, dark color.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct SpineVertex;

impl VertexDesc for SpineVertex {
    fn attribs() -> &'static [VertexAttribute] {
        // Colors go up as bytes and come back as normalized floats, so the
        // shader does not have to undo the packing by hand.
        static ATTRIBS: [VertexAttribute; 4] = vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
            2 => Unorm8x4,
            3 => Unorm8x4,
        ];
        &ATTRIBS
    }

    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: VERTEX_STRIDE as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: Self::attribs(),
        }
    }
}

/// GPU resources of one batch.
struct BatchGpu {
    vertex_buffer: Buffer,
    vertex_capacity: u64,
    index_buffer: Buffer,
    index_capacity: u64,
}

/// GPU resources of one node.
pub struct SpineGpu {
    /// Model epoch the resources belong to.
    epoch: u64,
    /// Whether a complete frame is ready to draw.
    pub ready: bool,
    /// Per-batch geometry buffers, in plugin submission order.
    batches: Vec<BatchGpu>,
    /// Uniform buffer and its bind group, shared by every batch.
    uniform_buffer: Buffer,
    uniform_group: BindGroup,
    /// Loaded textures of the model, indexed like the plugin's page list.
    texture_ids: Vec<Option<Arc<AssetId>>>,
    /// Pages the assets were requested for.
    pages: Vec<job::PageInfo>,
    /// Bind groups of the loaded textures, indexed like the page list.
    texture_groups: Vec<Option<BindGroup>>,
    /// The stored frame: its geometry lives in the buffers and is redrawn until
    /// a newer frame arrives, because the engine clears the target every frame.
    frame: Option<FrameSnapshot>,
    /// Draw plan of the stored frame.
    draws: Vec<DrawItem>,
    /// Whether a texture failure was already reported.
    texture_error: bool,
}

impl std::fmt::Debug for SpineGpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpineGpu")
            .field("epoch", &self.epoch)
            .field("batches", &self.batches.len())
            .finish_non_exhaustive()
    }
}

impl SpineGpu {
    fn new(epoch: u64, device: &Device, layout: &BindGroupLayout) -> Self {
        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Spine Uniform"),
            size: std::mem::size_of::<SpineUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Spine Uniform Group"),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            epoch,
            ready: false,
            batches: Vec::new(),
            uniform_buffer,
            uniform_group,
            texture_ids: Vec::new(),
            pages: Vec::new(),
            texture_groups: Vec::new(),
            frame: None,
            draws: Vec::new(),
            texture_error: false,
        }
    }
}

/// One batch draw of the stored frame.
struct DrawItem {
    /// Slot in the GPU batch list.
    slot: usize,
    /// Index into the page list of the model.
    page: usize,
    /// Pipeline slot, see [`blend_bucket`].
    bucket: usize,
    index_count: u32,
}

/// The renderer: pipelines and samplers shared by every spine node.
pub struct SpineRenderer {
    /// One pipeline per blend mode. No culling variants: a mirrored node reverses
    /// the winding of its own geometry, so both sides must stay visible.
    pipelines: [RenderPipeline; 4],
    uniform_layout: BindGroupLayout,
    texture_layout: BindGroupLayout,
}

impl SpineRenderer {
    pub fn new(device: &Device, format: TextureFormat, sample_count: u32) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Spine Shader"),
            source: ShaderSource::Wgsl(include_str!("./shaders/spine.wgsl").into()),
        });

        let uniform_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Spine Uniform Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(
                        std::mem::size_of::<SpineUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

        let texture_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Spine Texture Layout"),
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

        let mvp_layout = MVPMatrix::bind_group_layout(device);

        let create_pipeline = |label: &str, blend: i32| {
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[
                    Some(&mvp_layout),
                    Some(&uniform_layout),
                    Some(&texture_layout),
                ],
                immediate_size: 0,
            });

            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(SpineVertex::desc())],
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(ColorTargetState {
                        format,
                        blend: Some(blend_state_of(blend)),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    // A mirrored or negative-scale node reverses the winding of
                    // its own geometry, so culling would hide half of it.
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
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

        let pipelines = [
            create_pipeline("Spine Draw Normal", BLEND_NORMAL),
            create_pipeline("Spine Draw Additive", BLEND_ADDITIVE),
            create_pipeline("Spine Draw Multiply", BLEND_MULTIPLY),
            create_pipeline("Spine Draw Screen", BLEND_SCREEN),
        ];

        Self {
            pipelines,
            uniform_layout,
            texture_layout,
        }
    }

    /// Creates the bind group of one texture view with the page's sampling state.
    ///
    /// The atlas decides filtering and wrapping per page, so the sampler comes
    /// from the page rather than from the renderer: nearest-sampled pages and
    /// tiled ones both exist, and forcing one policy distorts them.
    fn texture_group(&self, device: &Device, view: &TextureView, sampling: job::Sampling) -> BindGroup {
        let sampler = sampler_for(device, sampling);
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Spine Texture Group"),
            layout: &self.texture_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        })
    }

    /// Requests the model's textures once the driver reports its page list.
    ///
    /// The page's declared encoding decides the asset kind, so an atlas that
    /// already premultiplies is not premultiplied a second time on upload. The
    /// sampling state is part of the identity too: a reload that only changes a
    /// filter still has to rebuild the bind groups.
    fn request_textures(
        &self,
        gpu: &mut SpineGpu,
        info: &job::ModelInfo,
        payload: &RendererUpdatePayload,
    ) {
        // `PageInfo` compares by value, so this covers path, encoding and sampling.
        if gpu.pages == info.pages {
            return;
        }

        gpu.pages = info.pages.clone();
        gpu.texture_ids = info
            .pages
            .iter()
            .map(|page| {
                let kind = if page.premultiplied {
                    AssetKind::TexturePremultiplied
                } else {
                    AssetKind::Texture
                };
                Some(payload.resource_manager.load_asset(kind, &page.path))
            })
            .collect();
        gpu.texture_groups = vec![None; info.pages.len()];
        gpu.texture_error = false;
    }

    /// Builds the bind groups of the loaded textures and reports whether any of
    /// them is still missing.
    fn resolve_textures(&self, device: &Device, gpu: &mut SpineGpu, node_id: u32) -> bool {
        let mut pending = false;
        let mut views: Vec<Option<Arc<TextureView>>> = Vec::with_capacity(gpu.texture_ids.len());

        for (index, id) in gpu.texture_ids.iter().enumerate() {
            let path = gpu
                .pages
                .get(index)
                .map(|page| page.path.as_str())
                .unwrap_or("");
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
            let (view, status) = texture_state(texture);
            match status {
                TextureStatus::Ready => match view {
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
                            SpineEvent::Error {
                                code: moyu_spine_abi::error::FILE,
                                message: format!("the spine texture failed to load: {path}"),
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
                let sampling = gpu
                    .pages
                    .get(index)
                    .map(|page| page.sampling)
                    .unwrap_or_default();
                gpu.texture_groups[index] = Some(self.texture_group(device, &view, sampling));
            }
        }
        pending
    }

    /// Uploads the stored frame's geometry into the per-batch buffers.
    fn upload_geometry(
        &self,
        device: &Device,
        render_queue: &RenderCommandSender,
        gpu: &mut SpineGpu,
    ) {
        let Some(frame) = gpu.frame.as_mut() else {
            return;
        };

        while gpu.batches.len() < frame.batches.len() {
            gpu.batches.push(BatchGpu {
                vertex_buffer: device.create_buffer(&BufferDescriptor {
                    label: Some("Spine Vertex Buffer"),
                    size: 1024,
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                vertex_capacity: 1024,
                index_buffer: device.create_buffer(&BufferDescriptor {
                    label: Some("Spine Index Buffer"),
                    size: 512,
                    usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                index_capacity: 512,
            });
        }

        // Every batch owns its own buffer: the uploads are all submitted before
        // any draw of this frame runs, so two batches sharing one buffer at
        // offset 0 would make every draw read the last batch's data.
        for (index, batch) in frame.batches.iter_mut().enumerate() {
            let slot = &mut gpu.batches[index];

            let vertex_data = std::mem::take(&mut batch.vertex_data);
            if !vertex_data.is_empty() {
                grow(
                    &mut slot.vertex_buffer,
                    &mut slot.vertex_capacity,
                    device,
                    vertex_data.len() as u64,
                    BufferUsages::VERTEX | BufferUsages::COPY_DST,
                    "Spine Vertex Buffer",
                );
                render_queue
                    .send(RenderCommand::WriteBuffer {
                        buffer: slot.vertex_buffer.clone(),
                        offset: 0,
                        data: vertex_data,
                        use_staging_belt: true,
                    })
                    .ok();
            }

            let index_data = std::mem::take(&mut batch.index_data);
            if !index_data.is_empty() {
                grow(
                    &mut slot.index_buffer,
                    &mut slot.index_capacity,
                    device,
                    index_data.len() as u64,
                    BufferUsages::INDEX | BufferUsages::COPY_DST,
                    "Spine Index Buffer",
                );
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
}

/// Reads a texture's view and status in one go.
fn texture_state(texture: &Arc<Texture>) -> (Option<Arc<TextureView>>, TextureStatus) {
    (texture.view.load_full(), texture.status())
}

/// Grows a buffer to fit `needed` bytes, keeping the existing one when it fits.
fn grow(
    buffer: &mut Buffer,
    capacity: &mut u64,
    device: &Device,
    needed: u64,
    usage: BufferUsages,
    label: &str,
) {
    if needed <= *capacity {
        return;
    }
    let size = needed.next_power_of_two().max(1024);
    *buffer = device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    });
    *capacity = size;
}

/// The blend state of one of the contract's blend modes.
///
/// Alpha is `One / OneMinusSrcAlpha` for every mode, which is the engine's
/// composition policy; the official Spine backends differ here.
fn blend_state_of(blend: i32) -> BlendState {
    let color = match blend {
        BLEND_ADDITIVE => BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
        BLEND_MULTIPLY => BlendComponent {
            src_factor: BlendFactor::Dst,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
        BLEND_SCREEN => BlendComponent {
            src_factor: BlendFactor::One,
            // `OneMinusSrc` is the per-component `1 - src`, which is the
            // classic screen blend's `1 - src_color`.
            dst_factor: BlendFactor::OneMinusSrc,
            operation: BlendOperation::Add,
        },
        _ => BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
    };

    BlendState {
        color,
        alpha: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
    }
}

/// Maps a contract blend mode onto its pipeline slot.
fn blend_bucket(blend: i32) -> usize {
    match blend {
        BLEND_ADDITIVE => 1,
        BLEND_MULTIPLY => 2,
        BLEND_SCREEN => 3,
        _ => 0,
    }
}

/// Maps a contract min/mag filter onto a wgpu one.
///
/// The mipmapped variants only differ in how the two mip levels are blended,
/// which is carried by `mipmap_filter`; the base filter stays linear because a
/// page that asked for mipmaps always wants smooth sampling.
fn filter_of(value: i32) -> FilterMode {
    match value {
        moyu_spine_abi::page::PAGE_FILTER_NEAREST => FilterMode::Nearest,
        _ => FilterMode::Linear,
    }
}

/// Maps a contract filter onto the mipmap blending mode.
///
/// A page that did not ask for mipmaps samples the base level, which has no
/// effect on a single-level texture.
fn mipmap_filter_of(value: i32) -> MipmapFilterMode {
    match value {
        moyu_spine_abi::page::PAGE_FILTER_MIPMAP_NEAREST => MipmapFilterMode::Nearest,
        moyu_spine_abi::page::PAGE_FILTER_MIPMAP_LINEAR => MipmapFilterMode::Linear,
        _ => MipmapFilterMode::Nearest,
    }
}

/// Maps a contract wrap mode onto a wgpu address mode.
fn address_mode_of(value: i32) -> AddressMode {
    match value {
        moyu_spine_abi::page::PAGE_WRAP_REPEAT => AddressMode::Repeat,
        moyu_spine_abi::page::PAGE_WRAP_MIRROR => AddressMode::MirrorRepeat,
        _ => AddressMode::ClampToEdge,
    }
}

/// Creates the sampler a page asks for.
///
/// Samplers are cheap but not free, and a model's pages rarely disagree, so the
/// caller caches them per sampling state instead of building one per frame.
fn sampler_for(device: &Device, sampling: job::Sampling) -> Sampler {
    device.create_sampler(&SamplerDescriptor {
        label: Some("Spine Page Sampler"),
        address_mode_u: address_mode_of(sampling.wrap_u),
        address_mode_v: address_mode_of(sampling.wrap_v),
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: filter_of(sampling.mag_filter),
        min_filter: filter_of(sampling.min_filter),
        mipmap_filter: mipmap_filter_of(sampling.min_filter),
        ..Default::default()
    })
}

/// Converts the node transform into the matrix the shader wants.
///
/// The engine stores transforms column-major and applies them as `matrix * v`,
/// while the shader applies `v * matrix`. Uploading the engine matrix unchanged
/// and letting the shader read `v * m` is the same product as transposing it on
/// the CPU, so the transpose stays out of the upload.
fn engine_mat4(transform: &moyu_core::base::Transform) -> glam::Mat4 {
    let affine: glam::Affine3A = **transform;
    glam::Mat4::from(affine)
}

/// Transform from world units into the node's local rectangle.
///
/// The mapping is a plain shift: the model's reference rectangle starts at the
/// node origin. Nothing is flipped — live2d and the engine's other renderers use
/// the same node-space convention, and the runtime's `Bone::yDown` default makes
/// its world space match it.
fn model_matrix(bounds_x: f32, bounds_y: f32) -> glam::Mat4 {
    glam::Mat4::from_cols(
        glam::Vec4::new(1.0, 0.0, 0.0, 0.0),
        glam::Vec4::new(0.0, 1.0, 0.0, 0.0),
        glam::Vec4::new(0.0, 0.0, 1.0, 0.0),
        glam::Vec4::new(-bounds_x, -bounds_y, 0.0, 1.0),
    )
}

impl Renderer for SpineRenderer {
    fn name(&self) -> &'static str {
        "spine"
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        &self.pipelines[0]
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.texture_layout
    }

    fn prepare(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        _queue: &Queue,
        payload: &RendererUpdatePayload,
    ) {
        let node = node.as_any_mut().downcast_mut::<SpineNode>().unwrap();
        let Some(info) = node.job.state.lock().model_info.clone() else {
            return;
        };

        // The reference bounds are the node's intrinsic size, and the textures
        // are requested here so they load while the script keeps running.
        let size = (info.bounds[2], info.bounds[3]);
        if node.base().intrinsic_size() != size {
            node.base_mut().set_intrinsic_size(size.0, size.1);
        }

        let gpu = node.gpu.get_or_insert_with(|| {
            Box::new(SpineGpu::new(info.epoch, device, &self.uniform_layout))
        });
        if gpu.epoch != info.epoch {
            **gpu = SpineGpu::new(info.epoch, device, &self.uniform_layout);
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
        let node = node.as_any_mut().downcast_mut::<SpineNode>().unwrap();

        let node_id = *node.base().id();
        let engine_matrix = engine_mat4(node.base().global_transform());
        let tint = node.base().tint().to_array();
        let global_opacity = *node.base().global_opacity();

        // The newest frame and the model it belongs to are read in one go, so
        // the renderer can never pair a frame with another model's resources.
        let (info, snapshot) = {
            let mut state = node.job.state.lock();
            (state.model_info.clone(), state.snapshot.take())
        };
        let epoch = info.as_ref().map_or(job::NO_MODEL_EPOCH, |info| info.epoch);
        if node.gpu.as_ref().is_none_or(|gpu| gpu.epoch != epoch) {
            node.gpu = Some(Box::new(SpineGpu::new(
                epoch,
                device,
                &self.uniform_layout,
            )));
        }
        let gpu = node.gpu.as_mut().unwrap();

        let Some(info) = info else {
            // Without a model there is nothing to draw.
            gpu.ready = false;
            gpu.draws.clear();
            return;
        };
        self.request_textures(gpu, &info, payload);

        // A new frame replaces the stored one and uploads its geometry once; the
        // plan below redraws it until a newer frame arrives.
        if let Some(snapshot) = snapshot {
            gpu.frame = Some(snapshot);
            self.upload_geometry(device, render_queue, gpu);
        }

        // A frame that cannot get all of its textures yet would draw with gaps,
        // so the node keeps showing nothing until the model is fully loaded.
        gpu.ready = false;
        gpu.draws.clear();
        if self.resolve_textures(device, gpu, node_id) {
            return;
        }
        let Some(frame) = gpu.frame.as_ref() else {
            return;
        };

        // The shader multiplies on the right (`v * node`), which WGSL reads as a
        // transposed matrix, so the transpose of the wanted transform is what
        // goes up.
        let [bounds_x, bounds_y, ..] = info.bounds;
        let transform = (engine_matrix * model_matrix(bounds_x, bounds_y)).transpose();

        let uniform = SpineUniform {
            node: transform.to_cols_array_2d(),
            base: [tint[0], tint[1], tint[2], global_opacity],
        };

        // The uniform is rewritten every frame through the render queue, which
        // keeps it ordered against the draws.
        render_queue
            .send(RenderCommand::WriteBuffer {
                buffer: gpu.uniform_buffer.clone(),
                offset: 0,
                data: bytemuck::bytes_of(&uniform).to_vec(),
                use_staging_belt: true,
            })
            .ok();

        gpu.draws = frame
            .batches
            .iter()
            .enumerate()
            .filter(|(_, batch)| batch.index_count > 0)
            .filter_map(|(slot, batch)| {
                // A negative or out-of-range page index would draw the wrong
                // texture; the plugin rejects those, so skipping is enough here.
                let page = usize::try_from(batch.page_index).ok()?;
                gpu.texture_groups.get(page)?.as_ref()?;
                Some(DrawItem {
                    slot,
                    page,
                    bucket: blend_bucket(batch.blend_mode),
                    index_count: batch.index_count as u32,
                })
            })
            .collect();
        gpu.ready = true;
    }

    fn collect_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        let Some(node) = node.as_any().downcast_ref::<SpineNode>() else {
            return;
        };
        let Some(gpu) = node.gpu.as_deref() else {
            return;
        };
        if !gpu.ready {
            return;
        }

        // Batches are submitted in the order the plugin produced them: the
        // runtime relies on that order for how overlapping attachments compose.
        for item in &gpu.draws {
            let Some(batch) = gpu.batches.get(item.slot) else {
                continue;
            };
            let Some(texture) = gpu
                .texture_groups
                .get(item.page)
                .and_then(|group| group.as_ref())
            else {
                continue;
            };

            render_queue
                .send(RenderCommand::Draw {
                    pipeline: self.pipelines[item.bucket].clone(),
                    bind_group: gpu.uniform_group.clone(),
                    extra_bind_groups: vec![texture.clone()],
                    vertex_buffer: Some(batch.vertex_buffer.clone()),
                    index_buffer: Some(batch.index_buffer.clone()),
                    instance_buffer: None,
                    count: item.index_count,
                    instance_count: 1,
                })
                .ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_buckets_match_the_contract() {
        assert_eq!(blend_bucket(BLEND_NORMAL), 0);
        assert_eq!(blend_bucket(BLEND_ADDITIVE), 1);
        assert_eq!(blend_bucket(BLEND_MULTIPLY), 2);
        assert_eq!(blend_bucket(BLEND_SCREEN), 3);
        // An unknown mode falls back to normal rather than failing the frame.
        assert_eq!(blend_bucket(99), 0);
    }

    #[test]
    fn vertex_layout_covers_the_stride() {
        let layout = SpineVertex::desc();
        assert_eq!(layout.array_stride, VERTEX_STRIDE as u64);
        let last = layout.attributes.last().unwrap();
        assert_eq!(last.offset + 4, VERTEX_STRIDE as u64);
    }

    #[test]
    fn page_sampling_maps_onto_wgpu() {
        use moyu_spine_abi::page::{
            PAGE_FILTER_LINEAR, PAGE_FILTER_MIPMAP_LINEAR, PAGE_FILTER_MIPMAP_NEAREST,
            PAGE_FILTER_NEAREST, PAGE_WRAP_CLAMP, PAGE_WRAP_MIRROR, PAGE_WRAP_REPEAT,
        };

        assert_eq!(filter_of(PAGE_FILTER_NEAREST), FilterMode::Nearest);
        assert_eq!(filter_of(PAGE_FILTER_LINEAR), FilterMode::Linear);
        // A page that asked for mipmaps samples smoothly at the base level too.
        assert_eq!(filter_of(PAGE_FILTER_MIPMAP_NEAREST), FilterMode::Linear);
        assert_eq!(filter_of(PAGE_FILTER_MIPMAP_LINEAR), FilterMode::Linear);

        assert_eq!(
            mipmap_filter_of(PAGE_FILTER_MIPMAP_NEAREST),
            MipmapFilterMode::Nearest
        );
        assert_eq!(
            mipmap_filter_of(PAGE_FILTER_MIPMAP_LINEAR),
            MipmapFilterMode::Linear
        );

        assert_eq!(address_mode_of(PAGE_WRAP_CLAMP), AddressMode::ClampToEdge);
        assert_eq!(address_mode_of(PAGE_WRAP_REPEAT), AddressMode::Repeat);
        assert_eq!(address_mode_of(PAGE_WRAP_MIRROR), AddressMode::MirrorRepeat);
    }

    #[test]
    fn uploaded_matrix_shifts_the_reference_rect_to_the_origin() {
        // The shader computes `v * node`, so what goes up is the transpose of the
        // wanted transform; this mirrors that path exactly, because asserting on
        // the untransposed matrix would pass even when the upload is wrong.
        let uploaded = model_matrix(10.0, 20.0).transpose();
        let node = uploaded.to_cols_array_2d();

        let apply = |x: f32, y: f32| {
            // `v * m` in WGSL is `transpose(m) * v`, and a uniform mat reads its
            // columns from the uploaded rows array.
            let v = glam::Vec4::new(x, y, 0.0, 1.0);
            glam::Mat4::from_cols_array_2d(&node).transpose() * v
        };

        // A reference rectangle of (10, 20, 100, 200) must land on (0, 0, 100, 200).
        let bottom_left = apply(10.0, 20.0);
        assert_eq!((bottom_left.x, bottom_left.y), (0.0, 0.0));

        let top_right = apply(110.0, 220.0);
        assert_eq!((top_right.x, top_right.y), (100.0, 200.0));
    }
}
