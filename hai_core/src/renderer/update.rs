use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

use super::walk::walk_nodes_top_bottom;
use crate::{node::NodeLike, sprite::SPRITE_INDICES, state::State};

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
