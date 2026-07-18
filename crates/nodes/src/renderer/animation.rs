use std::io::Cursor;
use std::sync::Arc;

use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::{AnimationDecoder, EncodableLayout, ImageDecoder};
use moyu_core::base::*;
use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::{Node, NodeBaseTrait, RendererUpdatePayload};
use moyu_core::traits::{RenderCommandSender, Renderer};
use moyu_pal::dir::assets_dir;
use moyu_resource::utils::premultiply_alpha;
use reiterator::Reiterate;
use wgpu::{util::DeviceExt, *};

use crate::nodes::{Animation, AnimationFormat, FrameIterator};
use crate::utils::{QUAD_INDICES, QUAD_INDICES_COUNT, QuadVertex, calculate_quad_vertices};

pub struct AnimationRenderer {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    index_buffer: Buffer,
    sampler: Sampler,
}

impl AnimationRenderer {
    pub fn new(device: &Device, config: &SurfaceConfiguration, sample_count: u32) -> Self {
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
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        // shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Animation Shader"),
            source: ShaderSource::Wgsl(include_str!("./shaders/simple.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Animation Pipeline Layout"),
            bind_group_layouts: &[
                Some(&MVPMatrix::bind_group_layout(device)),
                Some(&bind_group_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Animation Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[QuadVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
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

        // index buffers for each sprite are always the same.
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Animation Renderer Index Buffer"),
            // NINESLICE_INDICES includes RECTANGLE_INDICES, so we can use it for both,
            // and adjust the range when drawing.
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Animation Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            index_buffer,
            sampler,
        }
    }
}

impl Renderer for AnimationRenderer {
    fn name(&self) -> &'static str {
        "animation"
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    fn update(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        queue: &Queue,
        render_queue: &RenderCommandSender,
        payload: &RendererUpdatePayload,
    ) {
        let node = node.as_any_mut().downcast_mut::<Animation>().unwrap();

        // if there is a next_src, load it asyncly and store data to next_data,
        // clear current next_data if exists to improve performance
        if let Some(next_src) = node.next_src.take() {
            let _ = node.next_data.swap(None);
            let next_data = node.next_data.clone();
            let next_src_copy = next_src.clone();

            moyu_pal::task::spawn(async move {
                let asset_full_path = assets_dir().join(&next_src_copy).unwrap();

                let data = match moyu_pal::fs::read(&asset_full_path).await {
                    Ok(data) => data,
                    Err(e) => {
                        log::error!("Failed to read animation file: {}", e);
                        return Err(anyhow::anyhow!(
                            "Failed to read animation file: {}",
                            e.to_string()
                        ));
                    }
                };

                next_data.store(Some(Arc::new(data)));

                Ok(())
            });

            node.src = Some(next_src);
        }

        // if there is next_data, decode it and create texture and frames,
        // then reset next_data to None
        if let Some(next_data) = node.next_data.swap(None) {
            let format = node.format;
            // FIXME: avoid data copy
            let next_data = (&*next_data).to_owned();

            // decode animation frames
            let (frames, size) = match format {
                AnimationFormat::APNG => match PngDecoder::new(Cursor::new(next_data)) {
                    Ok(img) => {
                        let size = img.dimensions();

                        if !img.is_apng().unwrap_or(false) {
                            log::error!("The provided PNG is not an APNG");
                            (None, size)
                        } else {
                            match img.apng() {
                                Ok(img) => {
                                    let frames = img.into_frames().reiterate();
                                    (Some(frames), size)
                                }
                                Err(e) => {
                                    log::error!("Failed to decode APNG: {}", e);
                                    (None, size)
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to decode PNG: {}", e);
                        (None, (0, 0))
                    }
                },
                AnimationFormat::WEBP => match WebPDecoder::new(Cursor::new(next_data)) {
                    Ok(img) => {
                        let size = img.dimensions();

                        if !img.has_animation() {
                            log::error!("The provided WEBP is not an animated WEBP");
                            (None, size)
                        } else {
                            let frames = img.into_frames().reiterate();

                            (Some(frames), size)
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to decode WEBP: {}", e);
                        (None, (0, 0))
                    }
                },
            };

            // create new texture view
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Animation Texture"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
                label: None,
            });

            node.frames = frames.map(|v| FrameIterator(v));
            node.next_frame = None;
            node.view = Some(view);
            node.bind_group = Some(bind_group);

            // TODO: reset states

            // reset size to let it be recalculated
            node.base_mut().set_size(0, 0);
        }

        if node.view.is_some() {
            let view = node.view.as_ref().unwrap().clone();
            let texture = view.texture().clone();
            let size = texture.size();

            {
                // set size if not set
                let node_base = node.base();
                if node_base.width() == &0 && node_base.height() == &0 {
                    let [x1, y1, x2, y2] = node.area;
                    let size = texture.size();
                    node.base_mut().set_size(
                        (size.width as f32 * (x2 - x1)).round() as u32,
                        (size.height as f32 * (y2 - y1)).round() as u32,
                    );
                }
            }

            if node.base_mut().pop_update_vertices() {
                let vertices = calculate_quad_vertices(
                    node,
                    size.width as f32,
                    size.height as f32,
                    &[0., 0.],
                    &node.area,
                    &[1., 1.],
                );

                if node.vertex_buffer.is_none() {
                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Animation Vertex Buffer"),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
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

            if let Some(frames) = node.frames.as_mut() {
                let mut next_frame = node.next_frame.take();
                let mut current_frame: Option<(f64, image::ImageBuffer<image::Rgba<u8>, Vec<u8>>)> =
                    None;

                let mut reset_count = 0;

                loop {
                    if let Some((_, mut buffer)) = current_frame.take() {
                        let width = buffer.width();
                        let height = buffer.height();

                        premultiply_alpha(&mut buffer);

                        queue.write_texture(
                            view.texture().as_image_copy(),
                            buffer.as_bytes(),
                            wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(4 * width),
                                rows_per_image: Some(height),
                            },
                            size,
                        );

                        break;
                    }

                    // get next frame if not exists
                    if next_frame.is_none() {
                        next_frame = frames.next().and_then(|f| {
                            let Ok(f) = f.value else {
                                log::warn!("Failed to get next frame in animation.");
                                return None;
                            };

                            let delay = f.delay().numer_denom_ms();
                            let delay = (delay.0 as f64) / (delay.1 as f64) / 1000.0;

                            Some((payload.timestamp + delay, f.buffer().to_owned()))
                        });
                    }

                    // still none, means iterator ended, reset to beginning and continue
                    if next_frame.is_none() {
                        if reset_count > 0 {
                            // already reset once, avoid infinite loop
                            log::warn!("Animation frames iterator ended unexpectedly.");
                            break;
                        }
                        reset_count += 1;
                        frames.restart();
                        continue;
                    }

                    // next_frame must have been set here

                    let next_frame = next_frame.take().unwrap();

                    if payload.timestamp >= next_frame.0 {
                        // time to show this frame
                        current_frame = Some(next_frame);

                        // Since next_frame is consumed, we go to next iteration to get another frame,
                        // it will finally fall to the `else` branch below.
                        // By doing this, we can skip frames if the timestamp is too far ahead.
                        continue;
                    } else {
                        // not yet time, put it back to next_frame and break
                        node.next_frame = Some(next_frame);

                        if current_frame.is_none() {
                            // no frame to show yet, break
                            break;
                        }
                    }

                    // current_frame must have been set here
                }
            }
        }
    }

    fn begin(&self) {}
    fn finish(&self) {}

    fn collect_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        let node = node.as_any().downcast_ref::<Animation>().unwrap();
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
