use hai_pal::sync::{RwLock, RwLockReadGuard};
use std::sync::Arc;
use wgpu::util::StagingBelt;

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
    pub fn render(&mut self, state: &Arc<State>) -> Result<(), wgpu::SurfaceError> {
        let surface = state.surface.clone();
        let device = state.device.clone();
        let queue = state.queue.clone();
        let root_node = state.root_node.clone();

        let renderers = state.renderers.clone();
        let renderers = renderers.read();

        let surface_size = state.surface_size.lock().clone();

        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = {
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Command Encoder"),
            })
        };

        let mut belt_encoder = {
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Belt Command Encoder"),
            })
        };

        {
            let root_node = root_node.read();
            let upload_payload = RendererUpdatePayload {
                surface_size: surface_size.clone(),
            };

            let mut nodes: Vec<Arc<RwLock<dyn Node>>> = vec![];

            walk_nodes_top_bottom(&*root_node, &mut |child, parent| {
                let mut _child = child.write();
                _child.update_transform(parent.global_transform(), &surface_size, false);

                if let Some(child) = _child.try_as_renderable_mut() {
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
                }

                drop(_child);
                nodes.push(child);

                false
            });

            // FIXME: too many loops
            let childs: Vec<RwLockReadGuard<dyn Node>> = nodes.iter().map(|n| n.read()).collect();

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

            let childs: Vec<&dyn Renderable> = childs
                .iter()
                .filter_map(|n| n.try_as_renderable())
                .collect();

            for child in childs {
                let node_type = NodeType::node_type(child);

                let current_renderer = { renderers.get(node_type).unwrap() };

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

        queue.submit(
            std::iter::once(belt_encoder.finish()).chain(std::iter::once(encoder.finish())),
        );
        output.present();

        self.staging_belt.recall();

        Ok(())
    }
}
