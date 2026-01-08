use std::sync::{Arc, Weak};
use weak_table::WeakKeyHashMap;
use wgpu::{util::DeviceExt, *};

use moyu_core::base::{MVPMatrix, VertexDesc};
use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::{Node, NodeBaseTrait, RendererUpdatePayload};
use moyu_core::traits::{RenderCommandSender, Renderer};
use moyu_resource::types::{Asset, AssetId, AssetKind, Texture, TextureStatus};

use crate::nodes::{Sprite, SpriteMode};

pub const RECTANGLE_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    pub transform_0: [f32; 4],
    pub transform_1: [f32; 4],
    pub transform_2: [f32; 4],
    pub transform_3: [f32; 4],
    pub local_bounds: [f32; 4],
    pub uv_bounds: [f32; 4],
    pub tint: [f32; 4],
}

impl VertexDesc for SpriteInstance {
    fn attribs() -> &'static [wgpu::VertexAttribute] {
        static ATTRIBS: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
            2 => Float32x4, // transform_0
            3 => Float32x4, // transform_1
            4 => Float32x4, // transform_2
            5 => Float32x4, // transform_3
            6 => Float32x4, // local_bounds
            7 => Float32x4, // uv_bounds
            8 => Float32x4  // tint
        ];
        &ATTRIBS
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: Self::attribs(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadVertex {
    pub position: [f32; 2],
}

impl VertexDesc for QuadVertex {
    fn attribs() -> &'static [wgpu::VertexAttribute] {
        static ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![
            0 => Float32x2,
        ];
        &ATTRIBS
    }
}

fn calculate_sprite_instance(
    node: &dyn Node,
    tex_width: f32,
    tex_height: f32,
    origin: &[f32; 2],
    area: &[f32; 4],
    scale: &[f32; 2],
) -> SpriteInstance {
    let [x0, y0, x1, y1] = *area;
    let [x_scale, y_scale] = *scale;

    // scale size to fit area
    let width = tex_width * (x1 - x0);
    let height = tex_height * (y1 - y0);

    let w0 = origin[0] * tex_width;
    let w1 = w0 + width * x_scale;
    let h0 = origin[1] * tex_height;
    let h1 = h0 + height * y_scale;

    // Local rect in pixels relative to node origin
    let local_bounds = [w0, h0, w1, h1];

    // UV rect
    let uv_bounds = [x0, y0, x1, y1];

    // Global transform
    let global_transform = node.base().global_transform();

    // Tint
    let tint = node.base().tint();
    let opacity = node.base().global_opacity();
    let tint_vec = [
        tint.r as f32,
        tint.g as f32,
        tint.b as f32,
        tint.a as f32 * opacity,
    ];

    SpriteInstance {
        transform_0: [
            global_transform.x_axis.x,
            global_transform.x_axis.y,
            global_transform.x_axis.z,
            0.0,
        ],
        transform_1: [
            global_transform.y_axis.x,
            global_transform.y_axis.y,
            global_transform.y_axis.z,
            0.0,
        ],
        transform_2: [
            global_transform.z_axis.x,
            global_transform.z_axis.y,
            global_transform.z_axis.z,
            0.0,
        ],
        transform_3: [
            global_transform.w_axis.x,
            global_transform.w_axis.y,
            global_transform.w_axis.z,
            1.0,
        ],
        local_bounds,
        uv_bounds,
        tint: tint_vec,
    }
}

pub struct SpriteRenderer {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    quad_buffer: Buffer,
    index_buffer: Buffer,
    bind_group_map: WeakKeyHashMap<Weak<AssetId>, BindGroup>,
    last_sweep: u64,
}

impl SpriteRenderer {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
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
                buffers: &[QuadVertex::desc(), SpriteInstance::desc()],
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let quad_vertices = [
            QuadVertex {
                position: [0.0, 0.0],
            },
            QuadVertex {
                position: [0.0, 1.0],
            },
            QuadVertex {
                position: [1.0, 1.0],
            },
            QuadVertex {
                position: [1.0, 0.0],
            },
        ];
        let quad_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Quad Buffer"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // index buffers for each sprite are always the same.
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Renderer Index Buffer"),
            contents: bytemuck::cast_slice(RECTANGLE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            quad_buffer,
            index_buffer,
            bind_group_map: Default::default(),
            last_sweep: 0,
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
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
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
        device: &Device,
        _: &Queue,
        render_queue: &RenderCommandSender,
        payload: &RendererUpdatePayload,
    ) {
        // (image_logical_size * image_scale_factor) / (screen_logical_size * screen_scale_factor) * coordinate_factor
        // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed

        let node = node.as_any_mut().downcast_mut::<Sprite>().unwrap();

        if let Some(next_src) = node.next_src.take() {
            let texture_id = payload
                .resource_manager
                .load_asset(AssetKind::Texture, &next_src);
            node.next_texture_id.store(Some(texture_id));
        }

        // check if there's a pending texture change
        if let Some(next_texture_id) = node.next_texture_id.load().as_ref() {
            let texture = next_texture_id.asset_unchecked();
            let Asset::Texture(texture) = texture.as_ref() else {
                unreachable!("asset kind is not texture");
            };

            if TextureStatus::Ready == texture.status() {
                node.texture_id.store(node.next_texture_id.swap(None));
                // clean base node size, and re-assign it later
                node.base_mut().set_size(0, 0);
            }
        }

        if let Some(texture_id) = node.texture_id.load().as_ref() {
            let texture = texture_id.asset_unchecked();
            let Asset::Texture(texture) = texture.as_ref() else {
                unreachable!("asset kind is not texture");
            };

            if TextureStatus::Ready != texture.status() {
                return;
            }

            {
                // set size if not set
                let node_base = node.base();
                if node_base.width() == &0 && node_base.height() == &0 {
                    match node.mode {
                        SpriteMode::Normal => {
                            let [x1, y1, x2, y2] = node.area;
                            let (tex_width, tex_height) = texture.size();
                            node.base_mut().set_size(
                                (tex_width as f32 * (x2 - x1)).round() as u32,
                                (tex_height as f32 * (y2 - y1)).round() as u32,
                            );
                        }
                        SpriteMode::Nineslice => {
                            let target_width = node.target_width;
                            let target_height = node.target_height;
                            node.base_mut().set_size(target_width, target_height);
                        }
                    }
                }
            }

            let (tex_width, tex_height) = texture.size();
            let (tex_width, tex_height) = (tex_width as f32, tex_height as f32);

            if node.base_mut().pop_update_vertices() {
                let instances = match node.mode {
                    SpriteMode::Normal => vec![calculate_sprite_instance(
                        node,
                        tex_width,
                        tex_height,
                        &[0., 0.],
                        &node.area,
                        &[1., 1.],
                    )],
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
                        meshes.push(calculate_sprite_instance(
                            node,
                            tex_width,
                            tex_height,
                            &[0., 0.],
                            &[ax0, ay0, (ax0 + left), (ay0 + top)],
                            &[1., 1.],
                        ));

                        // left center
                        meshes.push(calculate_sprite_instance(
                            node,
                            tex_width,
                            tex_height,
                            &[0., btop],
                            &[ax0, (ay0 + top), (ax0 + left), (ay1 - bottom)],
                            &[1., center_v_scale],
                        ));

                        // left bottom
                        meshes.push(calculate_sprite_instance(
                            node,
                            tex_width,
                            tex_height,
                            &[0., btop + bcenter_v * center_v_scale],
                            &[ax0, (ay1 - bottom), (ax0 + left), ay1],
                            &[1., 1.],
                        ));

                        // center top
                        meshes.push(calculate_sprite_instance(
                            node,
                            tex_width,
                            tex_height,
                            &[bleft, 0.],
                            &[(ax0 + left), ay0, (ax1 - right), (ay0 + top)],
                            &[center_h_scale, 1.],
                        ));

                        // center center
                        meshes.push(calculate_sprite_instance(
                            node,
                            tex_width,
                            tex_height,
                            &[bleft, btop],
                            &[(ax0 + left), (ay0 + top), (ax1 - right), (ay1 - bottom)],
                            &[center_h_scale, center_v_scale],
                        ));

                        // center bottom
                        meshes.push(calculate_sprite_instance(
                            node,
                            tex_width,
                            tex_height,
                            &[bleft, btop + bcenter_v * center_v_scale],
                            &[(ax0 + left), (ay1 - bottom), (ax1 - right), ay1],
                            &[center_h_scale, 1.],
                        ));

                        // right top
                        meshes.push(calculate_sprite_instance(
                            node,
                            tex_width,
                            tex_height,
                            &[bleft + bcenter_h * center_h_scale, 0.],
                            &[(ax1 - right), ay0, ax1, (ay0 + top)],
                            &[1., 1.],
                        ));

                        // right center
                        meshes.push(calculate_sprite_instance(
                            node,
                            tex_width,
                            tex_height,
                            &[bleft + bcenter_h * center_h_scale, btop],
                            &[(ax1 - right), (ay0 + top), ax1, (ay1 - bottom)],
                            &[1., center_v_scale],
                        ));

                        // right bottom
                        meshes.push(calculate_sprite_instance(
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

                let buf = bytemuck::cast_slice(&instances);
                let current_size = node.instance_buffer.as_ref().map(|b| b.size()).unwrap_or(0);

                if node.instance_buffer.is_none() || current_size < buf.len() as u64 {
                    let instance_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Sprite Instance Buffer"),
                            contents: buf,
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        });

                    node.instance_buffer = Some(instance_buffer);
                } else {
                    render_queue
                        .send(RenderCommand::WriteBuffer {
                            buffer: node.instance_buffer.as_ref().unwrap().clone(),
                            offset: 0,
                            data: buf.to_vec(),
                            use_staging_belt: true,
                        })
                        .unwrap();
                }
            }

            // create bind group if not exist
            if !self.bind_group_map.contains_key(texture_id) {
                let bind_group = self.get_bind_group(device, &texture);
                self.bind_group_map.insert(texture_id.clone(), bind_group);
            }
        }

        let batch_timestamp = payload.timestamp as u64 / 10;
        if batch_timestamp > self.last_sweep {
            log::debug!("sweep bind_group_map");
            self.last_sweep = batch_timestamp;
            self.bind_group_map.remove_expired();
        }
    }

    fn begin(&self) {}
    fn finish(&self) {}

    fn collect_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        if !node.base().visible() {
            return;
        }

        let mut bind_group = None;
        let mut instance_buffer = None;
        let mut instance_count = 0;

        if let Some(sprite) = node.as_any().downcast_ref::<Sprite>() {
            if let Some(texture_id) = sprite.texture_id.load().as_ref() {
                bind_group = self.bind_group_map.get(texture_id);
                instance_buffer = sprite.instance_buffer.clone();
                instance_count = match sprite.mode {
                    SpriteMode::Normal => 1,
                    SpriteMode::Nineslice => 9,
                };
            }
        } else {
            unreachable!()
        }

        if let (Some(bind_group), Some(instance_buffer)) = (bind_group, instance_buffer) {
            render_queue
                .send(RenderCommand::Draw {
                    pipeline: self.pipeline.clone(),
                    bind_group: bind_group.clone(),
                    extra_bind_groups: vec![],
                    vertex_buffer: Some(self.quad_buffer.clone()),
                    index_buffer: Some(self.index_buffer.clone()),
                    instance_buffer: Some(instance_buffer),
                    count: 6 * instance_count,
                })
                .unwrap();
        }
    }
}
