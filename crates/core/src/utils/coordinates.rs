use crate::base::{Rect, get_scale_and_translate};
use crate::traits::Node;

/// Calculate the axis-aligned bounding box of a node after applying its global transformation.
/// Returns a Rect representing the bounding box in stage logical coordinates,
/// or None if the node has invalid dimensions or out of stage bounds.
pub fn calculate_bounding_box(
    node: &dyn Node,
    bound_width: f32,
    bound_height: f32,
) -> Option<Rect> {
    let base = node.base();
    let x = base.translate().x;
    let y = base.translate().y;
    let width = *base.width() as f32;
    let height = *base.height() as f32;

    // check for invalid dimensions
    if x + width <= 0.0 || y + height <= 0.0 || x >= bound_width || y >= bound_height {
        return None;
    }

    let gt = base.global_transform();
    let p0 = gt.transform_point3(glam::Vec3::new(x, y, 0.0));
    let p1 = gt.transform_point3(glam::Vec3::new(x + width, y, 0.0));
    let p2 = gt.transform_point3(glam::Vec3::new(x, y + height, 0.0));
    let p3 = gt.transform_point3(glam::Vec3::new(x + width, y + height, 0.0));

    // Calculate axis-aligned bounding box
    let min_x = p0.x.min(p1.x).min(p2.x).min(p3.x).max(0.0);
    let max_x = p0.x.max(p1.x).max(p2.x).max(p3.x);
    let min_y = p0.y.min(p1.y).min(p2.y).min(p3.y).max(0.0);
    let max_y = p0.y.max(p1.y).max(p2.y).max(p3.y);

    let width = (max_x - min_x).min(bound_width - min_x);
    let height = (max_y - min_y).min(bound_height - min_y);

    Some(Rect::new(min_x, min_y, width, height))
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
