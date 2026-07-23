use crate::nodes::NodeBase;

pub fn measure_children_layout_size(base: &NodeBase) -> (f32, f32) {
    let mut width = 0.0_f32;
    let mut height = 0.0_f32;

    for child in base.children() {
        let child = child.read();
        if !child.participates_in_parent_measure() {
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
