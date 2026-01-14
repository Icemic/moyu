use std::sync::Arc;

use glam::vec3a;

use crate::core::NodeLock;
use crate::traits::{FocusablePayload, Node};

use super::walk::walk_nodes_bottom_top;

#[derive(Debug)]
pub struct HitTestTarget {
    pub node: NodeLock,
    pub parent_ids: Vec<u32>,
}

impl PartialEq for HitTestTarget {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.node, &other.node)
    }
}

pub fn hit_test<'a>(
    _root_node: &NodeLock,
    global_logical_x: f32,
    global_logical_y: f32,
    upload_payload: &FocusablePayload,
) -> Option<HitTestTarget> {
    let root_node = _root_node.read();
    let mut focused_node = None;

    walk_nodes_bottom_top(
        &*root_node,
        &mut |child, _, parent_ids| {
            let child_ref = child.read();

            let (local_logical_x, local_logical_y) =
                get_local_logical_position(&*child_ref, global_logical_x, global_logical_y);

            let hit = match child_ref.as_focusable() {
                Some(focusable) => {
                    // check if pointer is over the node
                    let hit = focusable.contains(local_logical_x, local_logical_y, upload_payload);

                    if hit {
                        focused_node = Some(HitTestTarget {
                            node: child.clone(),
                            parent_ids: parent_ids.to_vec(),
                        });
                    }

                    hit
                }
                None => false,
            };

            hit
        },
        &[],
        true,
    );

    // at least hit the root node
    focused_node.or_else(|| {
        Some(HitTestTarget {
            node: _root_node.clone(),
            parent_ids: vec![],
        })
    })
}

#[inline]
pub fn get_local_logical_position(
    node: &dyn Node,
    global_logical_x: f32,
    global_logical_y: f32,
) -> (f32, f32) {
    let p = node
        .base()
        .global_transform()
        .inverse()
        .transform_point3a(vec3a(global_logical_x, global_logical_y, 1.0));

    (p.x, p.y)
}
