use hai_pal::sync::RwLock;
use log::debug;
use std::sync::Arc;
use winit::{dpi::PhysicalSize, event::WindowEvent};

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
    // let root_node = root_node.lock();

    let phy_size = PhysicalSize::new(state.physical_size.0, state.physical_size.1);
    let scale_factor = state.scale_factor;
    let logical_size = phy_size.to_logical::<f64>(scale_factor);

    drop(state);

    match event {
        WindowEvent::CursorMoved { position, .. } => {
            let global_logical_x = position.x / scale_factor;
            let global_logical_y = position.y / scale_factor;

            let root_node = root_node.lock();

            walk_nodes_bottom_top(&*root_node, &mut |child, parent| {
                let child_ref = child.lock();
                let hit = match NodeType::node_type(&*child_ref) {
                    "sprite" => {
                        let sprite = child_ref.as_any().downcast_ref::<Sprite>().unwrap();
                        // calculate relative coordinate
                        let parent_global_x =
                            parent.global_transform().tx * logical_size.width / 2.;
                        let parent_global_y =
                            parent.global_transform().ty * logical_size.height / 2.;

                        let relative_logical_x = (global_logical_x - parent_global_x).round();
                        let relative_logical_y = (global_logical_y - parent_global_y).round();

                        // check if pointer is over the sprite
                        let hit = sprite.contains(relative_logical_x, relative_logical_y);

                        (hit, Some(sprite.label.clone()))
                    }
                    _ => (false, None),
                };

                if hit.0 {
                    let mut current_focused_node = current_focused_node.lock();
                    *current_focused_node = Some(child.clone());
                    debug!("pointer is over {}", hit.1.unwrap());
                }

                hit.0
            });
            true
        }
        WindowEvent::CursorLeft { .. } => {
            let mut current_focused_node = current_focused_node.lock();
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
