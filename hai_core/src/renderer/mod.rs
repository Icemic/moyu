use std::sync::{Arc, Mutex};

use log::debug;
use wgpu::{
    util::DeviceExt, BindGroupLayout, Device, Queue, RenderPipeline, Surface, SurfaceConfiguration,
};
use winit::{dpi::PhysicalSize, event::WindowEvent, window::Window};

use crate::{
    node::{Node, NodeLike},
    sprite::SPRITE_INDICES,
    state::State,
    traits::Focusable,
    types::Vertex,
};

pub async fn create_surface(
    window: &Window,
    size: &PhysicalSize<u32>,
) -> (Surface, Device, Queue, SurfaceConfiguration) {
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
        .expect("No suitable GPU adapters found on the system.");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let adapter_info = adapter.get_info();
        println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
    }
    // or
    // let adapter = instance
    //     .enumerate_adapters(wgpu::Backends::all())
    //     .filter(|adapter| {
    //         // Check if this adapter supports our surface
    //         surface.get_preferred_format(&adapter).is_some()
    //     })
    //     .next()
    //     .unwrap();

    let limits = if !cfg!(target_arch = "wasm32") {
        wgpu::Limits::default()
    } else {
        wgpu::Limits::downlevel_webgl2_defaults()
    };

    // graphic card with specific backend
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits,
                label: None,
            },
            None, // Trace path
        )
        .await
        .expect("Unable to find a suitable GPU adapter.");

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

    (surface, device, queue, config)
}

pub fn prepare_pipeline(
    device: &Device,
    config: &SurfaceConfiguration,
) -> (RenderPipeline, BindGroupLayout) {
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

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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

    (render_pipeline, texture_bind_group_layout)
}

pub fn input<'a>(event: &WindowEvent, state: &Arc<Mutex<State<'a>>>) -> bool {
    let state = state.lock().unwrap();
    let root_node = state.root_node.clone();
    let current_focused_node = state.current_focused_node.clone();
    // let root_node = root_node.lock().unwrap();

    let phy_size = PhysicalSize::new(state.physical_size.0, state.physical_size.1);
    let scale_factor = state.scale_factor;
    let logical_size = phy_size.to_logical::<f64>(scale_factor);

    drop(state);

    match event {
        WindowEvent::CursorMoved { position, .. } => {
            let global_logical_x = position.x / scale_factor;
            let global_logical_y = position.y / scale_factor;

            let root_node = root_node.lock().unwrap();

            walk_nodes_bottom_top(&root_node, &mut |child, parent| {
                let mut child_ref = child.lock().unwrap();
                let hit = match &mut *child_ref {
                    NodeLike::Sprite(sprite) => {
                        // calculate relative coordinate
                        let parent_global_x =
                            parent.transform_to_global.tx * logical_size.width / 2.;
                        let parent_global_y =
                            parent.transform_to_global.ty * logical_size.height / 2.;

                        let relative_logical_x =
                            (global_logical_x - parent_global_x).round() as i32;
                        let relative_logical_y =
                            (global_logical_y - parent_global_y).round() as i32;

                        // check if pointer is over the sprite
                        let hit = sprite.contains(relative_logical_x, relative_logical_y);

                        (hit, Some(sprite.label.clone()))
                    }
                    _ => (false, None),
                };

                if hit.0 {
                    let mut current_focused_node = current_focused_node.lock().unwrap();
                    *current_focused_node = Some(child.clone());
                    debug!("[input] pointer is over {}", hit.1.unwrap());
                }

                hit.0
            });
            true
        }
        WindowEvent::CursorLeft { .. } => {
            let mut current_focused_node = current_focused_node.lock().unwrap();
            *current_focused_node = None;
            true
        }
        WindowEvent::MouseInput { .. } => {
            //
            println!("click");
            true
        }
        _ => false,
    }
}

pub fn update<'a>(state: &Arc<Mutex<State<'a>>>) {
    let state = state.lock().unwrap();
    let queue = state.pending_renderable.clone();
    let mut queue = queue.lock().unwrap();
    let root_node = state.root_node.clone();
    let root_node = root_node.lock().unwrap();
    let device = state.device.clone();
    let device = device.lock().unwrap();
    let texture_bind_group_layout = state.bind_group_layout.clone();
    let texture_bind_group_layout = texture_bind_group_layout.lock().unwrap();

    let phy_size = PhysicalSize::new(state.physical_size.0, state.physical_size.1);
    let scale_factor = state.scale_factor;
    let logical_size = phy_size.to_logical::<f64>(scale_factor);

    drop(state);

    // clear all update of last tick
    queue.clear();

    walk_nodes_top_bottom(&root_node, &mut |child, parent| {
        let mut child = child.lock().unwrap();
        match &mut *child {
            NodeLike::Sprite(sprite) => {
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &texture_bind_group_layout,
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

                sprite.calculate_transform(&parent.transform_to_global, logical_size, scale_factor);
                sprite.calculate_vertices(logical_size, scale_factor);

                let vertices = &sprite.vertices.unwrap();

                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

                let num_vertices = vertices.len() as u32;

                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(SPRITE_INDICES),
                    usage: wgpu::BufferUsages::INDEX,
                });
                let num_indices = SPRITE_INDICES.len() as u32;

                queue.push((
                    bind_group,
                    vertex_buffer,
                    index_buffer,
                    num_vertices,
                    num_indices,
                ));
            }
            NodeLike::Node(node) => {
                node.calculate_transform(&parent.transform_to_global, logical_size, scale_factor);
            }
        }
        false
    });
}

pub fn render<'a>(state: &Arc<Mutex<State<'a>>>) -> Result<(), wgpu::SurfaceError> {
    let state = state.lock().unwrap();
    let pending_renderable = state.pending_renderable.clone();
    let mut pending_renderable = pending_renderable.lock().unwrap();
    let surface = state.surface.clone();
    let surface = surface.lock().unwrap();
    let device = state.device.clone();
    let device = device.lock().unwrap();
    let queue = state.queue.clone();
    let queue = queue.lock().unwrap();
    let render_pipeline = state.render_pipeline.clone();
    let render_pipeline = render_pipeline.lock().unwrap();

    drop(state);

    let output = surface.get_current_texture()?;
    let view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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

        render_pass.set_pipeline(&render_pipeline);

        for (bind_group, vertex_buffer, index_buffer, _, num_indices) in pending_renderable.iter() {
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..*num_indices, 0, 0..1);
        }
    }

    // clear queue
    pending_renderable.clear();

    // submit will accept anything that implements IntoIter
    queue.submit(std::iter::once(encoder.finish()));
    output.present();

    Ok(())
}

/// walk through all node-like ones from top to bottom,
/// due that the depth should not big, recursive is acceptable
pub fn walk_nodes_top_bottom<T>(root_node: &Node, func: &mut T) -> bool
where
    // child, arr, parent_node  -> should_end
    T: FnMut(Arc<Mutex<NodeLike>>, &Node) -> bool,
{
    let children = &root_node.children;
    for child in children.iter() {
        let should_end = func(child.clone(), root_node);

        if should_end {
            return true;
        }

        let child = child.lock().unwrap();
        let node = match &*child {
            NodeLike::Sprite(sprite) => sprite,
            NodeLike::Node(n) => n,
        };

        if node.children.len() > 0 {
            let should_end = walk_nodes_top_bottom(node, func);
            if should_end {
                return true;
            }
        }
    }
    false
}

/// walk through all node-like ones from bottom to top,
/// due that the depth should not big, recursive is acceptable
pub fn walk_nodes_bottom_top<T>(root_node: &Node, func: &mut T) -> bool
where
    // child, arr, parent_node  -> should_end
    T: FnMut(Arc<Mutex<NodeLike>>, &Node) -> bool,
{
    let children = &root_node.children;
    for child in children.iter().rev() {
        {
            let child_ref = child.lock().unwrap();
            let node = match &*child_ref {
                NodeLike::Sprite(sprite) => sprite,
                NodeLike::Node(n) => n,
            };

            if node.children.len() > 0 {
                let should_end = walk_nodes_bottom_top(node, func);
                if should_end {
                    return true;
                }
            }
        }

        let should_end = func(child.clone(), root_node);
        if should_end {
            return true;
        }
    }
    false
}
