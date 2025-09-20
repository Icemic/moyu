use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::StagingBelt;
use wgpu::{util::DeviceExt, *};

use moyu_core::base::*;
#[cfg(feature = "video")]
use moyu_core::nodes::Video;
use moyu_core::traits::{Node, NodeBaseTrait, RendererUpdatePayload};
use moyu_core::utils::calculate::calculate_rect_vertices;
use moyu_core::utils::constants::{NINESLICE_INDICES, VIEWPORT_HEIGHT, VIEWPORT_WIDTH};
use moyu_core::{traits::Renderer, utils::constants::RECTANGLE_INDICES};
use moyu_resource::types::{Texture, TextureId, TextureStatus};

use crate::nodes::{Sprite, SpriteMode};

/// the number of vertices in a sprite is always 4.
// pub static NUM_VERTICES: u32 = 4;

static NUM_INDICES: u32 = RECTANGLE_INDICES.len() as u32;
static NUM_INDICES_NINESLICE: u32 = NINESLICE_INDICES.len() as u32;

pub struct SpriteRenderer {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    index_buffer: Buffer,
    bind_group_map: HashMap<Arc<TextureId>, BindGroup>,
}

impl SpriteRenderer {
    pub fn new(device: &Arc<Device>, config: &SurfaceConfiguration) -> Self {
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
                    ty: BindingType::Sampler(
                        // SamplerBindingType::Comparison is only for TextureSampleType::Depth
                        // SamplerBindingType::Filtering if the sample_type of the texture is:
                        //     TextureSampleType::Float { filterable: true }
                        // Otherwise you'll get an error.
                        SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        // shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Sprite Shader"),
            source: ShaderSource::Wgsl(include_str!("./shaders/default.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Sprite Pipeline Layout"),
            bind_group_layouts: &[&MVPMatrix::bind_group_layout(device), &bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Sprite Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[SpriteVertex::desc()],
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
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // index buffers for each sprite are always the same.
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Renderer Index Buffer"),
            // NINESLICE_INDICES includes RECTANGLE_INDICES, so we can use it for both,
            // and adjust the range when drawing.
            contents: bytemuck::cast_slice(NINESLICE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            pipeline,
            bind_group_layout,
            index_buffer,
            bind_group_map: Default::default(),
        }
    }
}

impl SpriteRenderer {
    fn get_bind_group(&mut self, device: &Device, texture: &Arc<Texture>) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        texture.view.load().as_ref().unwrap(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(
                        texture.sampler.load().as_ref().unwrap(),
                    ),
                },
            ],
            label: Some("bind_group"),
        })
    }
}

impl Renderer for SpriteRenderer {
    fn name(&self) -> &'static str {
        "sprite"
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
        _: &Arc<Queue>,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
        payload: &RendererUpdatePayload,
    ) {
        // (image_logical_size * image_scale_factor) / (screen_logical_size * screen_scale_factor) * coordinate_factor
        // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed

        let node = node.as_any_mut().downcast_mut::<Sprite>().unwrap();

        // check if there's a pending texture change
        if let Some(next_texture_id) = node.next_texture_id.load().as_ref() {
            let texture = payload.resource_manager.get_texture(next_texture_id);

            if TextureStatus::Ready == texture.status() {
                node.texture_id.store(Some(next_texture_id.clone()));
                node.next_texture_id.store(None);
            }
        }

        if let Some(texture_id) = node.texture_id.load().as_ref() {
            let texture = payload.resource_manager.get_texture(texture_id);

            if TextureStatus::Ready != texture.status() {
                return;
            }

            {
                // set size if not set
                let node = node.base_mut();
                if node.width() == &0 && node.height() == &0 {
                    let (tex_width, tex_height) = texture.size();
                    node.set_size(tex_width, tex_height);
                }
            }

            let (tex_width, tex_height) = texture.size();
            let (tex_width, tex_height) = (tex_width as f32, tex_height as f32);

            let width = tex_width / VIEWPORT_WIDTH;
            let height = tex_height / VIEWPORT_HEIGHT;

            if node.base_mut().pop_update_vertices() {
                let vertices = match node.mode {
                    SpriteMode::Normal => calculate_rect_vertices(
                        node,
                        width,
                        height,
                        &[0., 0.],
                        &node.area,
                        &[1., 1.],
                    )
                    .to_vec(),
                    SpriteMode::Nineslice => {
                        //
                        // (0,0)                            texture width
                        //     +-----------------------------------------------------------------------------+
                        //     |    (ax0,ay0)           |       area width                |                  |
                        //     |         +--------------+---------------------------------+------+           |
                        //     |         |      (1)     | top         (2)                 |  (3) |           |
                        //     |     ----+--------------|---------------------------------|------+---        |
                        //     |         |     left     |                                 |      |           |
                        //     |         |     (4)      |              (5)                |  (6) |  area     | texture height
                        //     |         |              |                                 |      | height    |
                        //     |         |              |                                 | right|           |
                        //     |     ----+--------------|---------------------------------+------+---        |
                        //     |         |      (7)     |              (8)         bottom |  (9) |           |
                        //     |         +--------------+---------------------------------+------+           |
                        //     |                        |                                 |   (ax1,ay1)      |
                        //     +-------------------------------------------------- --------------------------+
                        //                                                                                     (1,1)

                        let [ax0, ay0, ax1, ay1] = node.area;
                        let [bleft, btop, bright, bbottom] = node.bounds;

                        let bcenter_h = 1. - bleft - bright;
                        let bcenter_v = 1. - btop - bbottom;

                        // bounds relative to texture coordinates
                        let left = bleft * (ax1 - ax0);
                        let top = btop * (ay1 - ay0);
                        let right = bright * (ax1 - ax0);
                        let bottom = bbottom * (ay1 - ay0);
                        let center_h = (1. - bleft - bright) * (ax1 - ax0);
                        let center_v = (1. - btop - bbottom) * (ay1 - ay0);

                        // get the minimum width and height of the nine slice area (which means both edge and center are shrinked to 0)
                        let min_center_width = tex_width * center_h;
                        let min_center_height = tex_height * center_v;

                        let target_center_width =
                            (node.target_width as f32 - tex_width * (left + right)).max(0.);
                        let target_center_height =
                            (node.target_height as f32 - tex_height * (top + bottom)).max(0.);

                        let center_h_scale = target_center_width / min_center_width;
                        let center_v_scale = target_center_height / min_center_height;

                        // meshes to store vertices of 9 slices
                        let mut meshes = vec![];

                        // left top
                        meshes.extend(calculate_rect_vertices(
                            node,
                            tex_width,
                            tex_height,
                            &[0., 0.],
                            &[ax0, ay0, (ax0 + left), (ay0 + top)],
                            &[1., 1.],
                        ));

                        // left center
                        meshes.extend(calculate_rect_vertices(
                            node,
                            tex_width,
                            tex_height,
                            &[0., btop],
                            &[ax0, (ay0 + top), (ax0 + left), (ay1 - bottom)],
                            &[1., center_v_scale],
                        ));

                        // left bottom
                        meshes.extend(calculate_rect_vertices(
                            node,
                            tex_width,
                            tex_height,
                            &[0., btop + bcenter_v * center_v_scale],
                            &[ax0, (ay1 - bottom), (ax0 + left), ay1],
                            &[1., 1.],
                        ));

                        // center top
                        meshes.extend(calculate_rect_vertices(
                            node,
                            tex_width,
                            tex_height,
                            &[bleft, 0.],
                            &[(ax0 + left), ay0, (ax1 - right), (ay0 + top)],
                            &[center_h_scale, 1.],
                        ));

                        // center center
                        meshes.extend(calculate_rect_vertices(
                            node,
                            tex_width,
                            tex_height,
                            &[bleft, btop],
                            &[(ax0 + left), (ay0 + top), (ax1 - right), (ay1 - bottom)],
                            &[center_h_scale, center_v_scale],
                        ));

                        // center bottom
                        meshes.extend(calculate_rect_vertices(
                            node,
                            tex_width,
                            tex_height,
                            &[bleft, btop + bcenter_v * center_v_scale],
                            &[(ax0 + left), (ay1 - bottom), (ax1 - right), ay1],
                            &[center_h_scale, 1.],
                        ));

                        // right top
                        meshes.extend(calculate_rect_vertices(
                            node,
                            tex_width,
                            tex_height,
                            &[bleft + bcenter_h * center_h_scale, 0.],
                            &[(ax1 - right), ay0, ax1, (ay0 + top)],
                            &[1., 1.],
                        ));

                        // right center
                        meshes.extend(calculate_rect_vertices(
                            node,
                            tex_width,
                            tex_height,
                            &[bleft + bcenter_h * center_h_scale, btop],
                            &[(ax1 - right), (ay0 + top), ax1, (ay1 - bottom)],
                            &[1., center_v_scale],
                        ));

                        // right bottom
                        meshes.extend(calculate_rect_vertices(
                            node,
                            tex_width,
                            tex_height,
                            &[
                                bleft + bcenter_h * center_h_scale,
                                btop + bcenter_v * center_v_scale,
                            ],
                            &[(ax1 - right), (ay1 - bottom), ax1, ay1],
                            &[1., 1.],
                        ));

                        meshes
                    }
                };

                if node.vertex_buffer.is_none() {
                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Sprite Vertex Buffer"),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        });

                    node.vertex_buffer = Some(vertex_buffer);
                } else {
                    let buf = bytemuck::cast_slice(&vertices);
                    staging_belt
                        .write_buffer(
                            encoder,
                            node.vertex_buffer.as_ref().unwrap(),
                            0,
                            (buf.len() as u64).try_into().unwrap(),
                            device,
                        )
                        .copy_from_slice(buf);
                }
            }

            // create bind group if not exist
            if !self.bind_group_map.contains_key(texture_id) {
                let bind_group = self.get_bind_group(device, &texture);
                self.bind_group_map.insert(texture_id.clone(), bind_group);
            }
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

        let mut bind_group = None;
        let mut vertex_buffer = None;

        let mode;

        if let Some(sprite) = node.as_any().downcast_ref::<Sprite>() {
            mode = sprite.mode;
            if let Some(texture_id) = sprite.texture_id.load().as_ref() {
                bind_group = self.bind_group_map.get(texture_id);
                vertex_buffer = sprite.vertex_buffer.as_ref();
            }
        }
        // else if let Some(video) = node.as_any().downcast_ref::<Video>() {
        //     bind_group = video.texture.read().bind_group.as_ref().unwrap().clone();
        //     vertex_buffer = video.vertex_buffer.as_ref().unwrap();
        // }
        else {
            unreachable!()
        }

        if bind_group.is_some() && vertex_buffer.is_some() {
            render_pass.set_pipeline(self.render_pipeline());
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.set_bind_group(1, bind_group.unwrap(), &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.unwrap().slice(..));

            let num_indices = match mode {
                SpriteMode::Normal => NUM_INDICES,
                SpriteMode::Nineslice => NUM_INDICES_NINESLICE,
            };

            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        }
    }
}
