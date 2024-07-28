use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use glam::Vec3;
use hai_pal::env::{entry_dir, get_hai_env};
use hai_pal::sync::Mutex;
use huozi::constant::TEXTURE_SIZE;
use huozi::layout::Vertex;
use huozi::Huozi;
use log::{error, info};
use wgpu::util::StagingBelt;
use wgpu::Texture;
use wgpu::{util::DeviceExt, *};

use hai_core::base::MVPMatrix;
use hai_core::traits::Renderer;
use hai_core::traits::{Node, NodeBaseTrait, RendererUpdatePayload};
use hai_core::utils::calculate::tint_to_vec4;

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
    pub fn new(device: &Arc<Device>, config: &SurfaceConfiguration) -> Self {
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
            view_formats: &vec![],
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
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
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
        hai_pal::task::spawn(async move {
            let font_file = &get_hai_env().font_file;
            let asset_full_path = entry_dir()
                .join("assets/")
                .unwrap()
                .join(font_file)
                .unwrap();

            info!("Loading font file: {}", asset_full_path);

            let font_data = match hai_pal::fs::read(&asset_full_path).await {
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
        device: &Arc<Device>,
        _: &Arc<Queue>,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
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
        let tint = tint_to_vec4(tint, *opacity);

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
            let p =
                transform.transform_point3(Vec3::new(vertex.position[0], vertex.position[1], 1.0));

            vertex.position[0] = p.x;
            vertex.position[1] = p.y;

            // calculate color with tint and pre-multiplied alpha
            let color_r = vertex.color[0] * tint[0] * tint[3];
            let color_g = vertex.color[1] * tint[1] * tint[3];
            let color_b = vertex.color[2] * tint[2] * tint[3];
            let mut color_a = vertex.color[3] * tint[3];

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

            if vertices.len() == 0 || indices.len() == 0 {
                return;
            }

            let buf_vertices = bytemuck::cast_slice(&vertices);
            let buf_indices = bytemuck::cast_slice(&indices);
            staging_belt
                .write_buffer(
                    encoder,
                    node.vertex_buffer.as_ref().unwrap(),
                    0,
                    (buf_vertices.len() as u64).try_into().unwrap(),
                    &device,
                )
                .copy_from_slice(buf_vertices);
            staging_belt
                .write_buffer(
                    encoder,
                    node.index_buffer.as_ref().unwrap(),
                    0,
                    (buf_indices.len() as u64).try_into().unwrap(),
                    &device,
                )
                .copy_from_slice(buf_indices);
        }
    }
}

impl Renderer for TextRenderer {
    fn name(&self) -> &'static str {
        return "text";
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
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
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
            match huozi.layout_parse(
                &node.text,
                &node.layout_style,
                &node.text_style,
                huozi::ColorSpace::SRGB,
                None,
            ) {
                Ok((glyphs, total_width, total_height)) => {
                    // set layout size
                    node.total_width = total_width;
                    node.total_height = total_height;

                    node.glyph_vertices = glyphs;

                    node.base_mut().set_size(total_width, total_height);

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
                            wgpu::ImageCopyTexture {
                                aspect: wgpu::TextureAspect::All,
                                texture: &self.texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                            },
                            &sdf_bitmap,
                            wgpu::ImageDataLayout {
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
        if let Some(print_start_time) = &mut node.print_start_time {
            // Some(0.) means the print_start_time is not initialized
            if print_start_time == &0. {
                *print_start_time = payload.timestamp;
            }

            let (index, fade_from_index, progress) = match node.print_mode {
                TextPrintMode::Instant => {
                    node.print_start_time = None;
                    (node.glyph_vertices.len(), None, 1.0)
                }
                TextPrintMode::Typewriter => {
                    let total = node.glyph_vertices.len();
                    // current progress (in glyphs), it may be larger than the length of the text
                    let mut progress = (payload.timestamp - *print_start_time) * node.print_speed;
                    let index;
                    // check if the text is fully printed
                    if progress >= total as f64 {
                        progress = 1.;
                        index = total;
                        node.print_start_time = None;
                    } else {
                        // calculate the progress in the current glyph
                        index = progress.ceil() as usize;
                        progress = progress % 1.0;
                    }

                    (index, Some(index.saturating_sub(1)), progress as f32)
                }
                TextPrintMode::Printer => {
                    // max row + 1
                    let total = node.glyph_vertices.last().map(|g| g.row + 1).unwrap_or(0);
                    // current progress (in rows), it may be larger than the max row count of the text
                    let mut progress = (payload.timestamp - *print_start_time) * node.print_speed;
                    let index;
                    let fade_from_index;
                    // check if the text is fully printed
                    if progress >= total as f64 {
                        progress = 1.;
                        index = node.glyph_vertices.len();
                        fade_from_index = None;
                        node.print_start_time = None;
                    } else {
                        // calculate the progress in the current row
                        index = node
                            .glyph_vertices
                            .iter()
                            .position(|g| g.row as f64 >= progress)
                            .unwrap_or(node.glyph_vertices.len());
                        fade_from_index = node
                            .glyph_vertices
                            .iter()
                            .position(|g| (g.row + 1) as f64 >= progress);
                        progress = progress % 1.0;
                    }

                    (index, fade_from_index, progress as f32)
                }
            };

            self.update_vertices(
                device,
                queue,
                encoder,
                staging_belt,
                node,
                index,
                fade_from_index,
                progress,
            );
        } else if need_relayout {
            // re-render all when verti
            self.update_vertices(
                device,
                queue,
                encoder,
                staging_belt,
                node,
                node.glyph_vertices.len(),
                None,
                1.0,
            );
        }
    }

    fn begin(&self) {}
    fn finish(&self) {}

    fn render(
        &self,
        _: &Arc<Device>,
        _: &Arc<Queue>,
        render_pass: &mut RenderPass,
        node: &dyn Node,
    ) {
        if !node.base().visible() {
            return;
        }

        let node = node
            .as_any()
            .downcast_ref::<Text>()
            .expect("this node is not a text node");

        if node.vertex_buffer.is_some() && node.index_buffer.is_some() {
            let vertex_buffer = node.vertex_buffer.as_ref().unwrap();
            let index_buffer = node.index_buffer.as_ref().unwrap();
            let num_indices = node.num_indices;

            render_pass.set_pipeline(self.render_pipeline());
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.set_bind_group(1, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

            // FIXME: NUM_INDICES depends on which renderer the child matches.
            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        }
    }
}
