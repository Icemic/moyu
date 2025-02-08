use doufu_pal::sync::RwLock;
use std::sync::Arc;

use crate::traits::Node;

/// walk through all node-like ones from top to bottom,
/// due that the depth should not big, recursive is acceptable
pub fn walk_nodes_top_bottom<T>(root_node: &(dyn Node), func: &mut T) -> bool
where
    // child, arr, parent_node  -> should_end
    T: FnMut(Arc<RwLock<dyn Node>>, &(dyn Node)) -> bool,
{
    let children = root_node.base().children();
    for child in children.iter() {
        let should_end = func(child.clone(), root_node);

        if should_end {
            return true;
        }

        let child = child.read();

        if !child.base().children().is_empty() {
            let should_end = walk_nodes_top_bottom(&*child, func);
            if should_end {
                return true;
            }
        }
    }
    false
}

/// walk through all node-like ones from bottom to top,
/// due that the depth should not big, recursive is acceptable
pub fn walk_nodes_bottom_top<T>(
    root_node: &(dyn Node),
    func: &mut T,
    current_parent_ids: &[u32],
    only_interactive: bool,
) -> bool
where
    // child, parent_node, parent_ids  -> should_end
    T: FnMut(Arc<RwLock<dyn Node>>, &(dyn Node), &[u32]) -> bool,
{
    let children = root_node.base().children();
    for child in children.iter().rev() {
        {
            let child = child.read();

            // skip non-interactive nodes
            if !child.base().interactive() && only_interactive {
                continue;
            }

            if !child.base().children().is_empty() {
                let parent_ids = current_parent_ids
                    .iter()
                    .chain([child.base().id()])
                    .copied()
                    .collect::<Vec<_>>();

                let should_end =
                    walk_nodes_bottom_top(&*child, func, &parent_ids, only_interactive);
                if should_end {
                    return true;
                }
            }
        }

        let should_end = func(child.clone(), root_node, current_parent_ids);
        if should_end {
            return true;
        }
    }
    false
}
