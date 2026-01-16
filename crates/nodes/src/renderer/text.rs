use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use glam::vec3a;
use huozi::Huozi;
use huozi::constant::TEXTURE_SIZE;
use huozi::layout::Vertex;
use log::{error, info};
use moyu_pal::config::get_engine_config;
use moyu_pal::dir::assets_dir;
use moyu_pal::sync::Mutex;
use wgpu::Texture;
use wgpu::{util::DeviceExt, *};

use moyu_core::base::MVPMatrix;
use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::{Node, NodeBaseTrait, RenderCommandSender, RendererUpdatePayload};
use moyu_core::traits::{NodeEventSource, Renderer};

use crate::events::TextEvent;
use crate::nodes::{Text, TextPrintMode};

/// the number of vertices in a sprite is always 4.
// pub static NUM_VERTICES: u32 = 4;

pub struct TextRenderer {
    pipeline: RenderPipeline,
    texture: Texture,
    _sampler: Sampler,
    _view: TextureView,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    huozi: Arc<Mutex<Option<Huozi>>>,
    last_texture_version: AtomicU64,
}

impl TextRenderer {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let size = wgpu::Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_SIZE,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sdf texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let _view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let _sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("text_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&_sampler),
                },
            ],
            label: Some("text_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/text.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[&MVPMatrix::bind_group_layout(device), &bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Text Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
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
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let huozi = Arc::new(Mutex::new(None));

        Self {
            pipeline,
            texture,
            _sampler,
            _view,
            bind_group_layout,
            bind_group,
            huozi,
            last_texture_version: AtomicU64::new(0),
        }
    }

    pub fn init_huozi_from_data(&self, font_data: Vec<u8>) {
        let _huozi = Huozi::new(font_data);
        self.huozi.lock().replace(_huozi);
    }

    pub fn init_huozi_from_env(&self) {
        let huozi = self.huozi.clone();
        moyu_pal::task::spawn(async move {
            let font_file = &get_engine_config().font_file;
            let asset_full_path = assets_dir().join(font_file).unwrap();

            info!("Loading font file: {}", asset_full_path);

            let font_data = match moyu_pal::fs::read(&asset_full_path).await {
                Ok(data) => data,
                Err(e) => {
                    error!(
                        "Failed to read font file: {}, text rendering may not work.",
                        e
                    );
                    return;
                }
            };

            let _huozi = Huozi::new(font_data);
            huozi.lock().replace(_huozi);
        });
    }

    fn update_vertices(
        &self,
        device: &Device,
        _: &Queue,
        render_queue: &RenderCommandSender,
        node: &mut Text,
        last_index: usize,
        fade_from_index: Option<usize>,
        fade_progress: f32,
    ) {
        let glyphs = &node.glyph_vertices[..last_index];

        // transform to global
        let transform = node.base().global_transform();
        let tint = node.base().tint();
        let opacity = node.base().global_opacity();

        // assumes that each glyph has the same number of vertices in fill, stroke, and shadow
        let total_count_til_last_in_vertices = glyphs.iter().fold(0, |acc, g| acc + g.fill.len());

        // assumes that each glyph has the same number of vertices in fill, stroke, and shadow
        let fade_from_index_in_vertices = fade_from_index.map(|v| {
            node.glyph_vertices[..v]
                .iter()
                .fold(0, |acc, g| acc + g.fill.len())
        });

        let mut vertices: Vec<Vertex> = Vec::with_capacity(glyphs.len() * 4 * 3);
        let mut indices: Vec<u16> = Vec::with_capacity(glyphs.len() * 6);

        let mut index_offset = 0;

        if node.text_style.shadow.is_some() {
            for glyph in glyphs.iter() {
                vertices.extend(&glyph.shadow);
                indices.extend(glyph.indices.iter().map(|i| i + index_offset));

                index_offset += glyph.shadow.len() as u16;
            }
        }

        if node.text_style.stroke.is_some() {
            for glyph in glyphs.iter() {
                vertices.extend(&glyph.stroke);
                indices.extend(glyph.indices.iter().map(|i| i + index_offset));

                index_offset += glyph.stroke.len() as u16;
            }
        }

        for glyph in glyphs.iter() {
            vertices.extend(&glyph.fill);
            indices.extend(glyph.indices.iter().map(|i| i + index_offset));

            index_offset += glyph.fill.len() as u16;
        }

        for (i, vertex) in vertices.iter_mut().enumerate() {
            // FIXME: convertion between Vec2 and [f32; 2] may cause additional cost
            // y axis is inverted, so we need to invert it back, apply transform and invert it again
            let p = transform.transform_point3a(vec3a(vertex.position[0], vertex.position[1], 1.0));

            vertex.position[0] = p.x;
            vertex.position[1] = p.y;

            // calculate color with tint and pre-multiplied alpha
            let tint_a = tint.a * opacity;
            let color_r = vertex.color[0] * tint.r * tint_a;
            let color_g = vertex.color[1] * tint.g * tint_a;
            let color_b = vertex.color[2] * tint.b * tint_a;
            let mut color_a = vertex.color[3] * tint_a;

            if let Some(fade_from_index_in_vertices) = fade_from_index_in_vertices {
                if i % total_count_til_last_in_vertices >= fade_from_index_in_vertices {
                    color_a *= fade_progress;
                }
            }

            vertex.color = [color_r, color_g, color_b, color_a];
        }

        // drop the old buffer if the size is not enough
        if let Some(vertex_buffer) = &node.vertex_buffer {
            if vertex_buffer.size()
                < (vertices.len() * std::mem::size_of::<Vertex>()) as wgpu::BufferAddress
            {
                node.vertex_buffer = None;
            }
        }

        if node.vertex_buffer.is_none() {
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Text Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });

            node.vertex_buffer = Some(vertex_buffer);
            node.index_buffer = Some(index_buffer);
            node.num_indices = indices.len() as u32;
        } else {
            node.num_indices = indices.len() as u32;

            if vertices.is_empty() || indices.is_empty() {
                return;
            }

            let buf_vertices = bytemuck::cast_slice(&vertices);
            let buf_indices = bytemuck::cast_slice(&indices);

            render_queue
                .send(RenderCommand::WriteBuffer {
                    buffer: node.vertex_buffer.as_ref().unwrap().clone(),
                    offset: 0,
                    data: buf_vertices.to_vec(),
                    use_staging_belt: true,
                })
                .unwrap();

            render_queue
                .send(RenderCommand::WriteBuffer {
                    buffer: node.index_buffer.as_ref().unwrap().clone(),
                    offset: 0,
                    data: buf_indices.to_vec(),
                    use_staging_belt: true,
                })
                .unwrap();
        }
    }

    fn update_cursor_position(&self, node: &mut Text, next_index: usize) {
        // next_index is the end value of an open interval, but we need the end value of a closed interval,
        // so -1.
        let last_index = next_index as i32 - 1;
        if last_index < 0 {
            node.cursor_position = Some((0., 0.));
            return;
        }

        if let Some(glyph) = &node.glyph_vertices.get(last_index as usize) {
            let (next_x, next_y);
            if node.layout_style.direction == huozi::layout::LayoutDirection::Horizontal {
                // the last graph is top-right corner, this may be changed by huozi in the future
                next_x = (glyph.x + glyph.width) as f32 * glyph.scale_ratio as f32;
                next_y = glyph.y as f32 * glyph.scale_ratio as f32;
            } else {
                // bottom-left corner
                next_x = glyph.x as f32 * glyph.scale_ratio as f32;
                next_y = (glyph.y + glyph.height) as f32 * glyph.scale_ratio as f32;
            }
            node.cursor_position = Some((next_x, next_y));
        }
    }
}

impl Renderer for TextRenderer {
    fn name(&self) -> &'static str {
        "text"
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
        // update only when huozi is ready
        let mut huozi = self.huozi.lock();
        let huozi = match huozi.as_mut() {
            Some(huozi) => huozi,
            None => {
                return;
            }
        };

        let node = node.as_any_mut().downcast_mut::<Text>().unwrap();
        let need_relayout = node.base_mut().pop_update_vertices();

        if need_relayout {
            match huozi.layout_parse_with::<'<', '>'>(
                &node.segments,
                &node.layout_style,
                &node.text_style,
                huozi::ColorSpace::SRGB,
                None,
            ) {
                Ok((glyphs, ranges, total_width, total_height)) => {
                    // set layout size
                    node.total_width = total_width;
                    node.total_height = total_height;

                    node.glyph_vertices = glyphs;
                    node.glyph_ranges = ranges;

                    // Set size if size is different,
                    // this will trigger another relayout which is in fact unnecessary.
                    // What we need is to update the vertices only (base_mut().update() is called before
                    // child's update, so we have to emit update again).
                    if node.base_mut().width() != &total_width
                        || node.base_mut().height() != &total_height
                    {
                        node.base_mut().set_size(total_width, total_height);
                        node.base_mut().calculate_bounds();

                        // current transform matrix is not right because the size is not updated yet
                        // so we have to skip this tick until the next one when the transform matrix is updated
                        return;
                    }

                    // updates the sdf texture only when the image version is changed
                    let image_version = huozi.image_version();

                    if self.last_texture_version.load(Ordering::Relaxed) != image_version {
                        self.last_texture_version
                            .store(image_version, Ordering::Relaxed);

                        // update sdf texture
                        let sdf_bitmap = huozi.texture_image();
                        let dimensions = sdf_bitmap.dimensions();

                        let size = wgpu::Extent3d {
                            width: dimensions.0,
                            height: dimensions.1,
                            depth_or_array_layers: 1,
                        };

                        queue.write_texture(
                            wgpu::TexelCopyTextureInfo {
                                aspect: wgpu::TextureAspect::All,
                                texture: &self.texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                            },
                            sdf_bitmap,
                            wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(4 * sdf_bitmap.width()),
                                rows_per_image: Some(sdf_bitmap.height()),
                            },
                            size,
                        );
                    }
                }
                Err(err_msg) => {
                    error!("{}", err_msg);
                }
            }
        }

        // update vertices no matter if it is needed when it is printing
        if let Some(mut print_start_time) = node.print_start_time {
            // Some(0.) means the print_start_time is not initialized
            if print_start_time == 0. {
                print_start_time = payload.timestamp;
                node.print_start_time = Some(payload.timestamp);
                node.send_event(TextEvent::Start);
            }

            let (index, fade_from_index, progress, total_progress) = match node.print_mode {
                TextPrintMode::Instant => {
                    node.print_start_time = None;
                    (node.glyph_vertices.len(), None, 1.0, 1.0)
                }
                TextPrintMode::Typewriter => {
                    // current progress (in glyphs), it may be larger than the length of the text
                    let mut progress = (payload.timestamp - print_start_time) * node.print_speed;
                    let index;
                    let mut total_progress;

                    loop {
                        let last_range = node
                            .glyph_ranges
                            .get(node.current_range_index)
                            .map(|v| v.glyph_range.clone())
                            .unwrap_or_else(|| 0..node.glyph_vertices.len());
                        let total = last_range.end - last_range.start;
                        total_progress = progress / total as f64;
                        // check if the text is fully printed
                        if progress >= total as f64 {
                            // move to next segment
                            node.current_range_index += 1;

                            // reset progress for next segment
                            if node.current_range_index < node.glyph_ranges.len() {
                                node.print_start_time = Some(payload.timestamp);
                                progress -= total as f64;
                                continue;
                            }

                            // all segments printed
                            progress = 1.;
                            total_progress = 1.;
                            index = node.glyph_vertices.len();
                            node.print_start_time = None;
                        } else {
                            // calculate the progress in the current glyph
                            index = last_range.start + progress.ceil() as usize;
                            progress %= 1.0;
                        }
                        break;
                    }

                    (
                        index,
                        Some(index.saturating_sub(1)),
                        progress,
                        total_progress,
                    )
                }
                TextPrintMode::Printer => {
                    // current progress (in rows), it may be larger than the max row count of the text
                    let mut progress = (payload.timestamp - print_start_time) * node.print_speed;
                    let index;
                    let fade_from_index;
                    let mut total_progress;

                    loop {
                        let glyph_start = node
                            .glyph_ranges
                            .get(node.current_range_index)
                            .map(|v| v.glyph_range.start)
                            .unwrap_or(0);

                        let row_start = node
                            .glyph_vertices
                            .get(glyph_start)
                            .map(|g| g.row)
                            .unwrap_or(0);
                        // max row + 1
                        let row_end = node.glyph_vertices.last().map(|g| g.row + 1).unwrap_or(0);
                        let total = row_end - row_start;

                        total_progress = progress / total as f64;

                        // check if the text is fully printed
                        if progress >= total as f64 {
                            // move to next segment
                            node.current_range_index += 1;

                            // reset progress for next segment
                            if node.current_range_index < node.glyph_ranges.len() {
                                node.print_start_time = Some(payload.timestamp);
                                progress -= total as f64;
                                continue;
                            }

                            progress = 1.;
                            total_progress = 1.;
                            index = node.glyph_vertices.len();
                            fade_from_index = None;
                            node.print_start_time = None;
                        } else {
                            // calculate the progress in the current row
                            index = node
                                .glyph_vertices
                                .iter()
                                .position(|g| g.row as f64 - row_start as f64 >= progress)
                                .unwrap_or(node.glyph_vertices.len());
                            fade_from_index = node
                                .glyph_vertices
                                .iter()
                                .position(|g| (g.row + 1 - row_start) as f64 >= progress);
                            progress %= 1.0;
                        }

                        break;
                    }

                    (index, fade_from_index, progress, total_progress)
                }
            };

            node.send_event(TextEvent::Progress(total_progress));

            self.update_vertices(
                device,
                queue,
                render_queue,
                node,
                index,
                fade_from_index,
                progress as f32,
            );

            if progress >= 1.0 {
                node.send_event(TextEvent::Finish);
            }

            self.update_cursor_position(node, index);
        } else if need_relayout {
            // re-render all when vertices need update and not printing
            self.update_vertices(
                device,
                queue,
                render_queue,
                node,
                node.glyph_vertices.len(),
                None,
                1.0,
            );
            self.update_cursor_position(node, node.glyph_vertices.len());
        }
    }

    fn begin(&self) {}
    fn finish(&self) {}

    fn collect_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        let node = node
            .as_any()
            .downcast_ref::<Text>()
            .expect("this node is not a text node");

        if node.vertex_buffer.is_some() && node.index_buffer.is_some() && node.num_indices > 0 {
            let vertex_buffer = node.vertex_buffer.as_ref().unwrap();
            let index_buffer = node.index_buffer.as_ref().unwrap();
            let num_indices = node.num_indices;

            render_queue
                .send(RenderCommand::Draw {
                    pipeline: self.pipeline.clone(),
                    bind_group: self.bind_group.clone(),
                    extra_bind_groups: vec![],
                    vertex_buffer: Some(vertex_buffer.clone()),
                    index_buffer: Some(index_buffer.clone()),
                    instance_buffer: None,
                    count: num_indices,
                    instance_count: 1,
                })
                .unwrap();
        }
    }
}
