use std::{
    convert::TryInto,
    sync::{Arc, Mutex, MutexGuard},
};
use wgpu::util::StagingBelt;
use winit::dpi::PhysicalSize;

use crate::{
    state::State,
    traits::{Node, NodeType, Renderable, RendererUpdatePayload},
};

use super::{walk::walk_nodes_top_bottom, NUM_INDICES};

pub struct Renderer {
    staging_belt: StagingBelt,
}

impl Renderer {
    pub fn new() -> Self {
        let staging_belt = StagingBelt::new(0);
        Self { staging_belt }
    }
    pub fn render(&mut self, _state: &Arc<Mutex<State>>) -> Result<(), wgpu::SurfaceError> {
        let state = _state.lock().unwrap();
        let surface = state.surface.clone();
        let surface = surface.lock().unwrap();
        let device = state.device.clone();
        let queue = state.queue.clone();
        let root_node_arc = state.root_node.clone();

        let renderers = state.renderers.clone();
        let renderers = renderers.read().unwrap();
        let phy_size = PhysicalSize::new(state.physical_size.0, state.physical_size.1);
        let scale_factor = state.scale_factor;
        let logical_size = phy_size.to_logical::<f64>(scale_factor);

        drop(state);

        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = {
            let device = device.lock().unwrap();
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Command Encoder"),
            })
        };

        let mut belt_encoder = {
            let device = device.lock().unwrap();
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Belt Command Encoder"),
            })
        };

        {
            let root_node = root_node_arc.lock().unwrap();
            let upload_payload = RendererUpdatePayload {
                logical_size,
                scale_factor,
            };

            let mut nodes: Vec<Arc<Mutex<dyn Node>>> = vec![];

            walk_nodes_top_bottom(&*root_node, &mut |child, parent| {
                let mut _child = child.lock().unwrap();
                _child.calculate_transform(
                    parent.transform_to_global(),
                    logical_size,
                    scale_factor,
                );

                drop(_child);
                nodes.push(child);

                false
            });

            let mut childs: Vec<MutexGuard<dyn Node>> =
                nodes.iter_mut().map(|n| n.lock().unwrap()).collect();

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            let childs: Vec<&mut dyn Renderable> = childs
                .iter_mut()
                .filter_map(|n| n.try_as_renderable_mut())
                .collect();

            for child in childs {
                let node_type = NodeType::node_type(child);

                let current_renderer = { renderers.get(node_type).unwrap() };

                child.update(
                    &device,
                    &queue,
                    &mut belt_encoder,
                    &mut self.staging_belt,
                    current_renderer.bind_group_layout(),
                    &upload_payload,
                );

                render_pass.set_pipeline(current_renderer.render_pipeline());
                render_pass.set_index_buffer(
                    current_renderer.index_buffer().slice(..),
                    wgpu::IndexFormat::Uint16,
                );

                if let Some((bind_group, vertex_buffer)) = child.get_renderable() {
                    render_pass.set_bind_group(0, &bind_group, &[]);
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

                    // FIXME: NUM_INDICES depends on which renderer the child matches.
                    render_pass.draw_indexed(0..NUM_INDICES, 0, 0..1);
                }
            }
        }

        self.staging_belt.finish();

        let queue = queue.lock().unwrap();
        queue.submit(std::iter::once(belt_encoder.finish()));
        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.staging_belt.recall();

        Ok(())
    }
}
