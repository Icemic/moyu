use hai_pal::sync::RwLock;
use log::debug;
use std::sync::Arc;
use winit::event::WindowEvent;

use super::walk::walk_nodes_bottom_top;
use crate::{
    nodes::Sprite,
    state::State,
    traits::{Focusable, NodeType},
};

pub fn input(event: &WindowEvent, state: &Arc<RwLock<State>>) -> bool {
    let state = state.read();
    let root_node = state.root_node.clone();
    let current_focused_node = state.current_focused_node.clone();

    let surface_size = state.surface_size.lock();
    let (logical_width, logical_height) = surface_size.logical_size();
    let scale_factor = surface_size.scale_factor();

    drop(surface_size);
    drop(state);

    match event {
        WindowEvent::CursorMoved { position, .. } => {
            let global_logical_x = position.x / scale_factor;
            let global_logical_y = position.y / scale_factor;

            let root_node = root_node.read();

            walk_nodes_bottom_top(&*root_node, &mut |child, parent| {
                let child_ref = child.read();
                let hit = match NodeType::node_type(&*child_ref) {
                    "sprite" => {
                        let sprite = child_ref.as_any().downcast_ref::<Sprite>().unwrap();
                        // calculate relative coordinate
                        let parent_global_x = parent.global_transform().tx * logical_width / 2.;
                        let parent_global_y = parent.global_transform().ty * logical_height / 2.;

                        let relative_logical_x = (global_logical_x - parent_global_x).round();
                        let relative_logical_y = (global_logical_y - parent_global_y).round();

                        // check if pointer is over the sprite
                        let hit = sprite.contains(relative_logical_x, relative_logical_y);

                        (hit, Some(sprite.label.clone()))
                    }
                    _ => (false, None),
                };

                if hit.0 {
                    let mut current_focused_node = current_focused_node.write();
                    *current_focused_node = Some(child.clone());
                    debug!("pointer is over {}", hit.1.unwrap());
                }

                hit.0
            });
            true
        }
        WindowEvent::CursorLeft { .. } => {
            let mut current_focused_node = current_focused_node.write();
            *current_focused_node = None;
            true
        }
        WindowEvent::MouseInput { .. } => {
            //
            debug!("click");
            true
        }
        _ => false,
    }
}
