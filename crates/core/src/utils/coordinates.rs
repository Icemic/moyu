use crate::base::{Bound, Rect, get_scale_and_translate};
use crate::traits::Node;

/// Calculate the stage-space AABB of a node's layout rectangle.
pub fn calculate_layout_rect(
    node: &dyn Node,
    bound_width: f32,
    bound_height: f32,
) -> Rect {
    let base = node.base();
    let width = *base.width();
    let height = *base.height();

    if width <= 0.0 || height <= 0.0 {
        return Rect::default();
    }

    let bounds = Bound::new(0.0, 0.0, width, height)
        .transform(base.global_transform())
        .clamp(0.0, 0.0, bound_width, bound_height);

    if bounds.is_empty() {
        Rect::default()
    } else {
        bounds.into_rect()
    }
}

/// Calculate the physical coordinates of a rectangle on the surface,
/// given the stage and surface logical sizes and the scale factor.
/// Returns (x, y, width, height) in physical pixels relative to left-top corner of the surface (window).
pub fn calculate_surface_physical_coordinates(
    rect: &Rect,
    stage_logical_size: (f32, f32),
    surface_logical_size: (f32, f32),
    scale_factor: f32,
) -> (u32, u32, u32, u32) {
    let (scale, tx, ty) = get_scale_and_translate(
        stage_logical_size.0,
        stage_logical_size.1,
        surface_logical_size.0,
        surface_logical_size.1,
    );

    calculate_surface_physical_coordinates_by_scale_and_translate(rect, scale, tx, ty, scale_factor)
}

/// See `calculate_surface_physical_coordinates`
pub fn calculate_surface_physical_coordinates_by_scale_and_translate(
    rect: &Rect,
    scale: f32,
    tx: f32,
    ty: f32,
    scale_factor: f32,
) -> (u32, u32, u32, u32) {
    let x = ((rect.x() * scale + tx) * scale_factor) as u32;
    let y = ((rect.y() * scale + ty) * scale_factor) as u32;
    let w = ((rect.width() * scale) * scale_factor) as u32;
    let h = ((rect.height() * scale) * scale_factor) as u32;

    (x, y, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::Container;
    use crate::traits::NodeBaseTrait;

    fn update_node(node: &mut Container, parent: &Container) {
        node.base_mut().update(parent.base(), false);
    }

    #[test]
    fn layout_rect_applies_node_translation_once() {
        let parent = Container::new("parent".to_string());
        let mut node = Container::new("node".to_string());
        node.base_mut().set_layout_size(80.0, 60.0);
        node.base_mut().set_translate(100.0, 40.0);
        update_node(&mut node, &parent);

        assert_eq!(
            calculate_layout_rect(&node, 1920.0, 1080.0),
            Rect::new(100.0, 40.0, 80.0, 60.0)
        );
    }

    #[test]
    fn layout_rect_combines_parent_and_node_translation_once() {
        let root = Container::new("root".to_string());
        let mut parent = Container::new("parent".to_string());
        parent.base_mut().set_translate(50.0, 20.0);
        update_node(&mut parent, &root);

        let mut node = Container::new("node".to_string());
        node.base_mut().set_layout_size(80.0, 60.0);
        node.base_mut().set_translate(100.0, 40.0);
        update_node(&mut node, &parent);

        assert_eq!(
            calculate_layout_rect(&node, 1920.0, 1080.0),
            Rect::new(150.0, 60.0, 80.0, 60.0)
        );
    }

    #[test]
    fn layout_rect_combines_layout_position_and_translation_once() {
        let parent = Container::new("parent".to_string());
        let mut node = Container::new("node".to_string());
        node.base_mut().set_layout_size(80.0, 60.0);
        node.base_mut().set_layout_position(200.0, 120.0);
        node.base_mut().set_translate(15.0, 5.0);
        update_node(&mut node, &parent);

        assert_eq!(
            calculate_layout_rect(&node, 1920.0, 1080.0),
            Rect::new(215.0, 125.0, 80.0, 60.0)
        );
    }

    #[test]
    fn layout_rect_clamps_to_stage_bounds() {
        let parent = Container::new("parent".to_string());
        let mut node = Container::new("node".to_string());
        node.base_mut().set_layout_size(80.0, 60.0);
        node.base_mut().set_translate(-20.0, 1050.0);
        update_node(&mut node, &parent);

        assert_eq!(
            calculate_layout_rect(&node, 1920.0, 1080.0),
            Rect::new(0.0, 1050.0, 60.0, 30.0)
        );
    }

    #[test]
    fn layout_rect_respects_pivot_and_rotation() {
        let parent = Container::new("parent".to_string());
        let mut node = Container::new("node".to_string());
        node.base_mut().set_layout_size(80.0, 60.0);
        node.base_mut().set_translate(100.0, 100.0);
        node.base_mut().set_pivot(0.5, 0.5);
        node.base_mut().set_rotation(std::f32::consts::FRAC_PI_2);
        update_node(&mut node, &parent);

        let rect = calculate_layout_rect(&node, 1920.0, 1080.0);
        assert!((rect.x() - 70.0).abs() < 0.001);
        assert!((rect.y() - 60.0).abs() < 0.001);
        assert!((rect.width() - 60.0).abs() < 0.001);
        assert!((rect.height() - 80.0).abs() < 0.001);
    }

    #[test]
    fn layout_rect_is_empty_when_outside_stage() {
        let parent = Container::new("parent".to_string());
        let mut node = Container::new("node".to_string());
        node.base_mut().set_layout_size(80.0, 60.0);
        node.base_mut().set_translate(2000.0, 1200.0);
        update_node(&mut node, &parent);

        assert_eq!(
            calculate_layout_rect(&node, 1920.0, 1080.0),
            Rect::default()
        );
    }
}
