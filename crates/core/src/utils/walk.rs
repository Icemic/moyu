use crate::core::NodeLock;
use crate::traits::Node;

/// walk through all node-like ones from top to bottom,
/// due that the depth should not big, recursive is acceptable
pub fn walk_nodes_top_bottom<T>(root_node: &dyn Node, func: &mut T) -> bool
where
    // child, arr, parent_node  -> should_end
    T: FnMut(NodeLock, &dyn Node) -> bool,
{
    let children = root_node.base().children();
    for child in children.iter() {
        let should_end = func(child.clone(), root_node);

        if should_end {
            return true;
        }

        let child = child.read();

        if !child.base().children().is_empty() && child.base().visible() {
            let should_end = walk_nodes_top_bottom(&*child, func);
            if should_end {
                return true;
            }
        }
    }
    false
}

/// walk through all node-like ones from top to bottom with enter and leave callbacks
pub fn walk_nodes_enter_leave<E, L>(root_node: &dyn Node, enter: &mut E, leave: &mut L) -> bool
where
    E: FnMut(NodeLock, &dyn Node) -> bool,
    L: FnMut(NodeLock, &dyn Node),
{
    let children = root_node.base().children();
    for child in children.iter() {
        let should_end = enter(child.clone(), root_node);

        if should_end {
            return true;
        }

        {
            let child_ref = child.read();
            if !child_ref.base().children().is_empty() && child_ref.base().visible() {
                let should_end = walk_nodes_enter_leave(&*child_ref, enter, leave);
                if should_end {
                    return true;
                }
            }
        }

        leave(child.clone(), root_node);
    }
    false
}

/// walk through all node-like ones from bottom to top,
/// due that the depth should not big, recursive is acceptable
pub fn walk_nodes_bottom_top<T>(
    root_node: &dyn Node,
    func: &mut T,
    current_parent_ids: &[u32],
    only_interactive: bool,
) -> bool
where
    // child, parent_node, parent_ids  -> should_end
    T: FnMut(NodeLock, &dyn Node, &[u32]) -> bool,
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
