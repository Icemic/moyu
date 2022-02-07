use std::{cell::RefCell, rc::Rc};

use wgpu::{util::DeviceExt, BindGroupLayout};
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::*,
    window::Window,
};

use crate::{node::Node, sprite::SPRITE_INDICES};
use crate::{node::NodeLike, types::Vertex};

pub struct Renderer {
    pub logical_size: LogicalSize<f64>,
    pub physical_size: PhysicalSize<u32>,
    pub scale_factor: f64,
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: BindGroupLayout,

    root_node: Node<'static>,
    updated: Vec<(wgpu::BindGroup, wgpu::Buffer, wgpu::Buffer, u32, u32)>,
    // vertex_buffer: wgpu::Buffer,
    // num_vertices: u32,
    // index_buffer: wgpu::Buffer,
    // num_indices: u32,
    // bind_group: wgpu::BindGroup
}

impl Renderer {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        let logical_size = size.to_logical(scale_factor);

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        // or
        // let adapter = instance
        //     .enumerate_adapters(wgpu::Backends::all())
        //     .filter(|adapter| {
        //         // Check if this adapter supports our surface
        //         surface.get_preferred_format(&adapter).is_some()
        //     })
        //     .next()
        //     .unwrap();

        // graphic card with specific backend
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        // define how the surface creates its underlying SurfaceTextures
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            // width or height should not be 0 or it will cause crash
            width: size.width,
            height: size.height,
            // determines how to sync the surface with the display
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                        ty: wgpu::BindingType::Sampler(
                            // SamplerBindingType::Comparison is only for TextureSampleType::Depth
                            // SamplerBindingType::Filtering if the sample_type of the texture is:
                            //     TextureSampleType::Float { filterable: true }
                            // Otherwise you'll get an error.
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        // shader
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/default.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // create root node
        let root_node = Node::new(Some("Root Node"), Default::default(), Default::default());

        Self {
            physical_size: size,
            logical_size,
            scale_factor,
            surface,
            device,
            queue,
            config,
            render_pipeline,
            texture_bind_group_layout,
            root_node,
            updated: vec![],
        }
    }

    /// reset surface
    pub fn refresh(&mut self) {
        self.resize(self.physical_size, None);
    }

    // reconfigure the surface everytime the window's size changes
    pub fn resize(
        &mut self,
        new_size: winit::dpi::PhysicalSize<u32>,
        new_scale_factor: Option<f64>,
    ) {
        if new_size.width > 0 && new_size.height > 0 {
            self.physical_size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;

            if let Some(new_scale_factor) = new_scale_factor {
                self.scale_factor = new_scale_factor;
            }
            self.physical_size = new_size;
            self.logical_size = new_size.to_logical(self.scale_factor);

            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved {
                device_id,
                position,
                ..
            } => {
                // println!("mouse move to {}, {}", position.x, position.y);
                true
            }
            _ => false,
        }
    }

    pub fn get_root_node(&mut self) -> &mut Node<'static> {
        &mut self.root_node
    }

    pub fn update(&mut self) {
        // clear all update of last tick
        self.updated.clear();

        let device = &self.device;

        walk_nodes(&self.root_node, &mut |child, parent| {
            let mut child = child.borrow_mut();
            match &mut *child {
                NodeLike::Sprite(sprite) => {
                    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &self.texture_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&sprite.texture.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&sprite.texture.sampler),
                            },
                        ],
                        label: Some("bind_group"),
                    });

                    sprite.calculate_transform(
                        &parent.transform_to_global,
                        self.logical_size,
                        self.scale_factor,
                    );
                    sprite.calculate_vertices(self.logical_size, self.scale_factor);

                    let vertices = &sprite.vertices.unwrap();

                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Vertex Buffer"),
                            contents: bytemuck::cast_slice(vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                    let num_vertices = vertices.len() as u32;

                    let index_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Index Buffer"),
                            contents: bytemuck::cast_slice(SPRITE_INDICES),
                            usage: wgpu::BufferUsages::INDEX,
                        });
                    let num_indices = SPRITE_INDICES.len() as u32;

                    self.updated.push((
                        bind_group,
                        vertex_buffer,
                        index_buffer,
                        num_vertices,
                        num_indices,
                    ));
                }
                NodeLike::Node(node) => {
                    node.calculate_transform(
                        &parent.transform_to_global,
                        self.logical_size,
                        self.scale_factor,
                    );
                }
            }
        });
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);

            for (bind_group, vertex_buffer, index_buffer, _, num_indices) in self.updated.iter() {
                render_pass.set_bind_group(0, &bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..*num_indices, 0, 0..1);
            }
        }

        // clear queue
        self.updated.clear();

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

/// walk through all node-like ones,
/// due that the depth should not big, recursive is acceptable
pub fn walk_nodes<'a, T>(root_node: &Node<'a>, func: &mut T)
where
    // child, arr, parent_node
    T: FnMut(Rc<RefCell<NodeLike<'a>>>, &Node<'a>),
{
    let children = &root_node.children;
    for child in children {
        func(child.clone(), root_node);
        let child = child.borrow();
        let node = match &*child {
            NodeLike::Sprite(sprite) => sprite,
            NodeLike::Node(n) => n,
        };

        if node.children.len() > 0 {
            walk_nodes(node, func);
        }
    }
}
