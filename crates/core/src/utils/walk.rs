use crate::core::NodeLock;
use crate::traits::{Node, ShadowKind};

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
