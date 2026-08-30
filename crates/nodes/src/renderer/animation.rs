use std::sync::Arc;

use moyu_core::base::*;
use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::{Node, NodeBaseTrait, RendererUpdatePayload};
use moyu_core::traits::{RenderCommandSender, Renderer};
use moyu_image::{AnimationDecoder, AnimationFormat as ImageAnimationFormat};
use moyu_pal::dir::assets_dir;
use wgpu::{util::DeviceExt, *};

use crate::nodes::{Animation, AnimationFormat};
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
                buffers: &[Some(QuadVertex::desc())],
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

    fn prepare(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        _: &Queue,
        _: &RendererUpdatePayload,
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

        // if there is next_data, decode it and create texture and decoder,
        // then reset next_data to None
        if let Some(next_data) = node.next_data.swap(None) {
            let format = node.format;
            let format = match format {
                AnimationFormat::APNG => ImageAnimationFormat::Apng,
                AnimationFormat::WEBP => ImageAnimationFormat::WebP,
            };
            let decoder = match AnimationDecoder::new((*next_data).clone(), format) {
                Ok(decoder) => decoder,
                Err(error) => {
                    log::error!("Failed to decode animation: {}", error);
                    return;
                }
            };
            let size = (decoder.width(), decoder.height());

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

            node.decoder = Some(decoder);
            node.next_frame = None;
            node.upload_buffer.clear();
            node.view = Some(view);
            node.bind_group = Some(bind_group);
            node.base_mut().mark_update_vertices();
        }

        if let Some(view) = &node.view {
            let size = view.texture().size();
            let [x1, y1, x2, y2] = node.area;
            node.base_mut().set_intrinsic_size(
                size.width as f32 * (x2 - x1),
                size.height as f32 * (y2 - y1),
            );
        }
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

        if node.view.is_some() {
            let view = node.view.as_ref().unwrap().clone();
            let texture = view.texture().clone();
            let size = texture.size();

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

            if let Some(decoder) = node.decoder.as_mut() {
                let mut next_frame = node.next_frame.take();
                let mut show_current_frame = false;
                let mut reset_count = 0;

                loop {
                    if show_current_frame {
                        if let Err(error) =
                            decoder.write_premultiplied_frame(&mut node.upload_buffer)
                        {
                            log::warn!("Failed to prepare animation frame: {}", error);
                            break;
                        }
                        queue.write_texture(
                            view.texture().as_image_copy(),
                            &node.upload_buffer,
                            wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(4 * decoder.width()),
                                rows_per_image: Some(decoder.height()),
                            },
                            size,
                        );
                        break;
                    }

                    if next_frame.is_none() {
                        match decoder.advance() {
                            Ok(Some(delay)) => {
                                next_frame = Some(payload.timestamp + delay.as_secs_f64());
                            }
                            Ok(None) if reset_count == 0 => {
                                reset_count += 1;
                                if let Err(error) = decoder.restart() {
                                    log::warn!("Failed to restart animation: {}", error);
                                    break;
                                }
                                continue;
                            }
                            Ok(None) => {
                                log::warn!("Animation decoder ended unexpectedly.");
                                break;
                            }
                            Err(error) => {
                                log::warn!("Failed to decode animation frame: {}", error);
                                break;
                            }
                        }
                    }

                    let next_frame = next_frame.take().expect("animation frame is scheduled");
                    if payload.timestamp >= next_frame {
                        show_current_frame = true;
                    } else {
                        node.next_frame = Some(next_frame);
                        break;
                    }
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
