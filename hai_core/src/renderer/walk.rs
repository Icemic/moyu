use std::sync::{Arc, Mutex};

use crate::node::{Node, NodeLike};

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
