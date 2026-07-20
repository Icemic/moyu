use std::sync::Arc;

use glam::vec3a;

use crate::core::NodeLock;
use crate::traits::{FocusablePayload, Node};

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
    let focused_node = hit_test_children(
        root_node.as_ref(),
        global_logical_x,
        global_logical_y,
        upload_payload,
        &[],
    );

    // at least hit the root node
    focused_node.or_else(|| {
        Some(HitTestTarget {
            node: _root_node.clone(),
            parent_ids: vec![],
        })
    })
}

fn hit_test_children(
    parent: &dyn Node,
    global_logical_x: f32,
    global_logical_y: f32,
    payload: &FocusablePayload,
    current_parent_ids: &[u32],
) -> Option<HitTestTarget> {
    for child in parent.base().children_in_paint_order().iter().rev() {
        let child_ref = child.read();

        if !child_ref.base().interactive() {
            continue;
        }

        let (local_logical_x, local_logical_y) =
            get_local_logical_position(child_ref.as_ref(), global_logical_x, global_logical_y);

        let focusable = child_ref.as_focusable();
        let contains_children = focusable.is_none_or(|focusable| {
            focusable.contains_children(local_logical_x, local_logical_y, payload)
        });

        if !child_ref.base().children().is_empty() && contains_children {
            let parent_ids = current_parent_ids
                .iter()
                .chain([child_ref.base().id()])
                .copied()
                .collect::<Vec<_>>();

            if let Some(target) = hit_test_children(
                child_ref.as_ref(),
                global_logical_x,
                global_logical_y,
                payload,
                &parent_ids,
            ) {
                return Some(target);
            }
        }

        if focusable
            .is_some_and(|focusable| focusable.contains(local_logical_x, local_logical_y, payload))
        {
            return Some(HitTestTarget {
                node: child.clone(),
                parent_ids: current_parent_ids.to_vec(),
            });
        }
    }

    None
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
