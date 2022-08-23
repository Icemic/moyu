use std::sync::{Arc, Mutex};

use crate::traits::Node;

/// walk through all node-like ones from top to bottom,
/// due that the depth should not big, recursive is acceptable
pub fn walk_nodes_top_bottom<T>(root_node: &(dyn Node), func: &mut T) -> bool
where
    // child, arr, parent_node  -> should_end
    T: FnMut(Arc<Mutex<dyn Node>>, &(dyn Node)) -> bool,
{
    let children = root_node.children();
    for child in children.iter() {
        let should_end = func(child.clone(), root_node);

        if should_end {
            return true;
        }

        let child = child.lock().unwrap();

        if child.children().len() > 0 {
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
pub fn walk_nodes_bottom_top<T>(root_node: &(dyn Node), func: &mut T) -> bool
where
    // child, arr, parent_node  -> should_end
    T: FnMut(Arc<Mutex<dyn Node>>, &(dyn Node)) -> bool,
{
    let children = root_node.children();
    for child in children.iter().rev() {
        {
            let child = child.lock().unwrap();

            if child.children().len() > 0 {
                let should_end = walk_nodes_bottom_top(&*child, func);
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
