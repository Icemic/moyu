use std::sync::Arc;

use moyu_core::base::*;
use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::{Node, NodeBaseTrait, NodeEventSource, RendererUpdatePayload};
use moyu_core::traits::{RenderCommandSender, Renderer};
use moyu_pal::dir::assets_dir;
use moyu_video::PixelFormat;
use wgpu::{util::DeviceExt, *};

use crate::events::VideoEvent;
use crate::nodes::Video;
use crate::utils::{QUAD_INDICES, QUAD_INDICES_COUNT, QuadVertex, calculate_quad_vertices};
use moyu_video::PlaybackState;

pub struct VideoRenderer {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    index_buffer: Buffer,
    sampler: Sampler,
    /// Uniform buffer for VideoParams (format: u32 padded to 16 bytes)
    params_buffer_i420: Buffer,
    params_buffer_nv12: Buffer,
    params_buffer_rgba: Buffer,
    params_buffer_bgra: Buffer,
}

impl VideoRenderer {
    pub fn new(device: &Device, config: &SurfaceConfiguration, sample_count: u32) -> Self {
        // YUV bind group layout: Y texture, U texture, V texture, sampler
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
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
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("video_bind_group_layout"),
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Video Shader"),
            source: ShaderSource::Wgsl(include_str!("./shaders/video.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Video Pipeline Layout"),
            bind_group_layouts: &[
                Some(&MVPMatrix::bind_group_layout(device)),
                Some(&bind_group_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Video Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[QuadVertex::desc()],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Video Renderer Index Buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: BufferUsages::INDEX,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Video Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });

        // Pre-create uniform buffers for all formats (16 bytes each, padded u32)
        let params_buffer_i420 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Video Params I420"),
            contents: bytemuck::cast_slice(&[0u32, 0, 0, 0]),
            usage: BufferUsages::UNIFORM,
        });
        let params_buffer_nv12 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Video Params NV12"),
            contents: bytemuck::cast_slice(&[1u32, 0, 0, 0]),
            usage: BufferUsages::UNIFORM,
        });
        let params_buffer_rgba = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Video Params RGBA"),
            contents: bytemuck::cast_slice(&[2u32, 0, 0, 0]),
            usage: BufferUsages::UNIFORM,
        });
        let params_buffer_bgra = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Video Params BGRA"),
            contents: bytemuck::cast_slice(&[3u32, 0, 0, 0]),
            usage: BufferUsages::UNIFORM,
        });

        Self {
            pipeline,
            bind_group_layout,
            index_buffer,
            sampler,
            params_buffer_i420,
            params_buffer_nv12,
            params_buffer_rgba,
            params_buffer_bgra,
        }
    }

    /// Create or re-create YUV plane textures and bind group when dimensions or format change.
    fn ensure_textures(
        &self,
        node: &mut Video,
        device: &Device,
        width: u32,
        height: u32,
        format: PixelFormat,
    ) {
        if node.current_dimensions == Some((width, height))
            && node.current_format == Some(format)
            && node.bind_group.is_some()
        {
            return;
        }

        let chroma_width = (width + 1) / 2;
        let chroma_height = (height + 1) / 2;

        let y_usage = match format {
            PixelFormat::External => {
                TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT
            }
            _ => TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        };

        let (y_format, u_format, y_width, y_height, u_width, u_height, v_width, v_height) =
            match format {
                PixelFormat::I420 => (
                    TextureFormat::R8Unorm,
                    TextureFormat::R8Unorm,
                    width,
                    height,
                    chroma_width,
                    chroma_height,
                    chroma_width,
                    chroma_height,
                ),
                PixelFormat::Nv12 => (
                    TextureFormat::R8Unorm,
                    TextureFormat::Rg8Unorm,
                    width,
                    height,
                    chroma_width,
                    chroma_height,
                    1,
                    1,
                ),
                PixelFormat::Rgba | PixelFormat::Bgra => (
                    TextureFormat::Rgba8Unorm,
                    TextureFormat::R8Unorm,
                    width,
                    height,
                    1,
                    1,
                    1,
                    1,
                ),
                PixelFormat::External => (
                    TextureFormat::Rgba8Unorm,
                    TextureFormat::R8Unorm,
                    width,
                    height,
                    1,
                    1,
                    1,
                    1,
                ),
            };

        let tex_y = device.create_texture(&TextureDescriptor {
            label: Some("Video Y Plane"),
            size: Extent3d {
                width: y_width,
                height: y_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: y_format,
            usage: y_usage,
            view_formats: &[],
        });

        let tex_u = device.create_texture(&TextureDescriptor {
            label: Some("Video U Plane"),
            size: Extent3d {
                width: u_width,
                height: u_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: u_format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let tex_v = device.create_texture(&TextureDescriptor {
            label: Some("Video V Plane"),
            size: Extent3d {
                width: v_width,
                height: v_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view_y = tex_y.create_view(&TextureViewDescriptor::default());
        let view_u = tex_u.create_view(&TextureViewDescriptor::default());
        let view_v = tex_v.create_view(&TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view_y),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&view_u),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&view_v),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: match format {
                        PixelFormat::I420 => self.params_buffer_i420.as_entire_binding(),
                        PixelFormat::Nv12 => self.params_buffer_nv12.as_entire_binding(),
                        PixelFormat::Rgba => self.params_buffer_rgba.as_entire_binding(),
                        PixelFormat::Bgra => self.params_buffer_bgra.as_entire_binding(),
                        PixelFormat::External => self.params_buffer_rgba.as_entire_binding(),
                    },
                },
            ],
            label: Some("video_bind_group"),
        });

        node.view_y = Some(view_y);
        node.view_u = Some(view_u);
        node.view_v = Some(view_v);
        node.bind_group = Some(bind_group);
        node.current_dimensions = Some((width, height));
        node.current_format = Some(format);

        // Reset size to force recalculation
        node.base_mut().set_intrinsic_size(0.0, 0.0);
    }

    /// Upload YUV frame data to GPU textures.
    fn upload_frame(&self, node: &Video, queue: &Queue, frame: &moyu_video::DecodedFrame) {
        let width = frame.width;
        let height = frame.height;

        match frame.format {
            PixelFormat::I420 => {
                // Y plane: full resolution
                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture: node.view_y.as_ref().unwrap().texture(),
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    &frame.planes[0],
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(frame.strides[0]),
                        rows_per_image: Some(height),
                    },
                    Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );

                let chroma_width = (width + 1) / 2;
                let chroma_height = (height + 1) / 2;

                // U plane: half resolution
                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture: node.view_u.as_ref().unwrap().texture(),
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    &frame.planes[1],
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(frame.strides[1]),
                        rows_per_image: Some(chroma_height),
                    },
                    Extent3d {
                        width: chroma_width,
                        height: chroma_height,
                        depth_or_array_layers: 1,
                    },
                );

                // V plane: half resolution
                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture: node.view_v.as_ref().unwrap().texture(),
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    &frame.planes[2],
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(frame.strides[2]),
                        rows_per_image: Some(chroma_height),
                    },
                    Extent3d {
                        width: chroma_width,
                        height: chroma_height,
                        depth_or_array_layers: 1,
                    },
                );
            }
            PixelFormat::Nv12 => {
                // Y plane: full resolution
                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture: node.view_y.as_ref().unwrap().texture(),
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    &frame.planes[0],
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(frame.strides[0]),
                        rows_per_image: Some(height),
                    },
                    Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );

                // UV plane: upload directly as RG8Unorm (shader handles split)
                let chroma_width = (width + 1) / 2;
                let chroma_height = (height + 1) / 2;

                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture: node.view_u.as_ref().unwrap().texture(),
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    &frame.planes[1],
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(frame.strides[1]),
                        rows_per_image: Some(chroma_height),
                    },
                    Extent3d {
                        width: chroma_width,
                        height: chroma_height,
                        depth_or_array_layers: 1,
                    },
                );

                // V texture is a dummy for NV12, no upload needed
            }
            PixelFormat::Rgba | PixelFormat::Bgra => {
                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture: node.view_y.as_ref().unwrap().texture(),
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    &frame.planes[0],
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(frame.strides[0]),
                        rows_per_image: Some(height),
                    },
                    Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );
            }
            PixelFormat::External => {
                #[cfg(web)]
                {
                    let Some(video_frame) = frame.external_frame() else {
                        log::warn!("External video frame is missing its VideoFrame handle");
                        return;
                    };

                    let video_frame = match video_frame.clone() {
                        Ok(video_frame) => video_frame,
                        Err(err) => {
                            log::warn!("Failed to clone VideoFrame for GPU upload: {:?}", err);
                            return;
                        }
                    };

                    let source = CopyExternalImageSourceInfo {
                        source: ExternalImageSource::VideoFrame(video_frame),
                        origin: Origin2d::ZERO,
                        flip_y: false,
                    };

                    queue.copy_external_image_to_texture(
                        &source,
                        CopyExternalImageDestInfo {
                            texture: node.view_y.as_ref().unwrap().texture(),
                            mip_level: 0,
                            origin: Origin3d::ZERO,
                            aspect: TextureAspect::All,
                            color_space: PredefinedColorSpace::Srgb,
                            premultiplied_alpha: false,
                        },
                        Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                    );

                    // The queue captures the source image at submission time; close the
                    // temporary clone immediately so the browser doesn't GC it later.
                    if let ExternalImageSource::VideoFrame(video_frame) = source.source {
                        video_frame.close();
                    }
                }

                #[cfg(not(web))]
                {
                    log::warn!("External video frame upload is only available on web targets");
                }
            }
        }
    }
}

impl Renderer for VideoRenderer {
    fn name(&self) -> &'static str {
        "video"
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    fn prepare(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        _: &Queue,
        _: &RendererUpdatePayload,
    ) {
        let node = node.as_any_mut().downcast_mut::<Video>().unwrap();

        // Handle pending source change: load file asynchronously
        if let Some(next_src) = node.next_src.take() {
            let _ = node.next_data.swap(None);
            let next_data = node.next_data.clone();
            let next_src_copy = next_src.clone();

            moyu_pal::task::spawn(async move {
                let asset_full_path = assets_dir().join(&next_src_copy).unwrap();

                let data = match moyu_pal::fs::read(&asset_full_path).await {
                    Ok(data) => data,
                    Err(e) => {
                        log::error!("Failed to read video file: {}", e);
                        return Err(anyhow::anyhow!("Failed to read video file: {}", e));
                    }
                };

                next_data.store(Some(Arc::new(data)));
                Ok(())
            });

            node.src = Some(next_src);
        }

        // Handle loaded data: initialize the player
        if let Some(next_data) = node.next_data.swap(None) {
            let data = (&*next_data).to_owned();
            // Pass the full src path so the player can detect codec from filename
            let src_path = node.src.as_deref();

            let mut player = node.player.lock();
            match player.load(data, src_path) {
                Ok(()) => {
                    player.set_loop(node.looping);
                    player.set_volume(node.volume);
                    player.set_muted(node.muted);
                    if node.auto_play {
                        player.play().unwrap();
                    }
                    log::info!(
                        "Video loaded: {:?}, size={:?}, duration={:?}",
                        node.src,
                        player.video_size(),
                        player.duration()
                    );
                }
                Err(e) => {
                    log::error!("Failed to load video: {}", e);
                }
            }
        }

        let current_state = {
            let mut player = node.player.lock();
            player.tick();
            player.state()
        };
        if current_state != node.prev_state {
            let state_str = match current_state {
                PlaybackState::Idle => "idle",
                PlaybackState::Loading => "loading",
                PlaybackState::Playing => "playing",
                PlaybackState::Paused => "paused",
                PlaybackState::Stopped => "stopped",
                PlaybackState::Ended => "ended",
                PlaybackState::Error => "error",
            };
            node.prev_state = current_state;
            node.send_event(VideoEvent::StateChange(state_str.to_string()));
            if current_state == PlaybackState::Ended {
                node.send_event(VideoEvent::Ended);
            }
        }

        if current_state == PlaybackState::Ended {
            return;
        }

        let frame = node.player.lock().current_frame();
        if let Some(frame) = frame {
            self.ensure_textures(node, device, frame.width, frame.height, frame.format);
            node.base_mut()
                .set_intrinsic_size(frame.width as f32, frame.height as f32);
        }
    }

    fn update(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        queue: &Queue,
        render_queue: &RenderCommandSender,
        _payload: &RendererUpdatePayload,
    ) {
        let node = node.as_any_mut().downcast_mut::<Video>().unwrap();

        let frame = {
            let player = node.player.lock();
            player.current_frame()
        };

        if let Some(frame) = frame {
            self.upload_frame(node, queue, &frame);
        }

        // Update vertex buffer if needed
        if node.bind_group.is_some() {
            if let Some((width, height)) = node.current_dimensions {
                if node.base_mut().pop_update_vertices() {
                    let vertices = calculate_quad_vertices(
                        node,
                        width as f32,
                        height as f32,
                        &[0., 0.],
                        &[0., 0., 1., 1.],
                        &[1., 1.],
                    );

                    if node.vertex_buffer.is_none() {
                        let vertex_buffer =
                            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("Video Vertex Buffer"),
                                contents: bytemuck::cast_slice(&vertices),
                                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                            });
                        node.vertex_buffer = Some(vertex_buffer);
                    } else {
                        let buf = bytemuck::bytes_of(&vertices);
                        render_queue
                            .send(RenderCommand::WriteBuffer {
                                buffer: node.vertex_buffer.as_ref().unwrap().clone(),
                                offset: 0,
                                data: buf.to_vec(),
                                use_staging_belt: true,
                            })
                            .unwrap();
                    }
                }
            }
        }
    }

    fn begin(&self) {}
    fn finish(&self) {}

    fn collect_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        let node = node.as_any().downcast_ref::<Video>().unwrap();
        if let (Some(bind_group), Some(vertex_buffer)) = (&node.bind_group, &node.vertex_buffer) {
            render_queue
                .send(RenderCommand::Draw {
                    pipeline: self.pipeline.clone(),
                    bind_group: bind_group.clone(),
                    extra_bind_groups: vec![],
                    vertex_buffer: Some(vertex_buffer.clone()),
                    index_buffer: Some(self.index_buffer.clone()),
                    instance_buffer: None,
                    count: QUAD_INDICES_COUNT,
                    instance_count: 1,
                })
                .unwrap();
        }
    }
}
