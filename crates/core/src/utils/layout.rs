use crate::nodes::NodeBase;

pub fn measure_children_layout_size(base: &NodeBase) -> (f32, f32) {
    let mut width = 0.0_f32;
    let mut height = 0.0_f32;

    for child in base.children() {
        let child = child.read();
        if child.base().exclude_from_layout() {
            continue;
        }

        let child_base = child.base();
        let (child_width, child_height) = child_base.layout_size();
        let child_pivot = child_base.pivot();
        width = width.max(child_base.translate().x - child_pivot.x * child_width + child_width);
        height = height.max(child_base.translate().y - child_pivot.y * child_height + child_height);
    }

    (width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::Container;
    use crate::traits::{Node, NodeBaseTrait};

    #[test]
    fn excluded_children_do_not_contribute_to_parent_size() {
        let mut parent = Container::default();
        let managed = Container::default().into_node_lock();
        let excluded = Container::default().into_node_lock();

        managed.write().base_mut().set_layout_size(100.0, 40.0);
        excluded.write().base_mut().set_layout_size(300.0, 200.0);
        excluded.write().base_mut().set_exclude_from_layout(true);
        parent.base_mut().add_child(managed);
        parent.base_mut().add_child(excluded);

        assert_eq!(measure_children_layout_size(parent.base()), (100.0, 40.0));
    }
}
