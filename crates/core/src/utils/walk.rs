use crate::core::NodeLock;
use crate::traits::{Node, ShadowKind};

/// Walk the complete logical tree without applying rendering visibility or shadow rules.
pub fn walk_nodes_enter_leave_logical<E, L>(root_node: &dyn Node, enter: &mut E, leave: &mut L)
where
    E: FnMut(NodeLock, &dyn Node),
    L: FnMut(NodeLock, &dyn Node),
{
    let children = root_node.base().children();
    for child in children.iter() {
        enter(child.clone(), root_node);

        {
            let child_ref = child.read();
            if !child_ref.base().children().is_empty() {
                walk_nodes_enter_leave_logical(child_ref.as_ref(), enter, leave);
            }
        }

        leave(child.clone(), root_node);
    }
}

/// walk through all node-like ones from top to bottom with enter and leave callbacks
pub fn walk_nodes_enter_leave<E, L, R>(root_node: &dyn Node, enter: &mut E, leave: &mut L)
where
    E: FnMut(NodeLock, &dyn Node) -> R,
    L: FnMut(NodeLock, &dyn Node, R),
{
    let children = root_node.base().children();
    for child in children.iter() {
        let r = enter(child.clone(), root_node);

        {
            let child_ref = child.read();
            if !child_ref.base().children().is_empty()
                && child_ref.base().visible()
                && !child_ref.shadowed(ShadowKind::Rendering)
            {
                walk_nodes_enter_leave(child_ref.as_ref(), enter, leave);
            }
        }

        leave(child.clone(), root_node, r);
    }
}
