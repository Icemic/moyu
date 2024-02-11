use crate::base::*;
use crate::traits::Node;

#[inline]
pub fn calculate_rect_vertices(
    node: &dyn Node,
    tex_width: f32,
    tex_height: f32,
    area: &[f32; 4],
) -> [SpriteVertex; 4] {
    let [x0, y0, x1, y1] = area.to_owned();

    // scale size to fit area
    let width = tex_width * (x1 - x0);
    let height = tex_height * (y1 - y0);

    let global_transform = node.base().global_transform();

    let a = global_transform.x_axis.x;
    let b = global_transform.x_axis.y;
    let c = global_transform.y_axis.x;
    let d = global_transform.y_axis.y;

    // add addtional offset to move from center to left top
    let tx = global_transform.z_axis.x;
    let ty = global_transform.z_axis.y;

    let anchor = node.base().anchor();

    let w1 = -anchor.x * width;
    let w0 = w1 + width;
    let h1 = -anchor.y * height;
    let h0 = h1 + height;

    // left top
    let p0x = a * w1 + c * h1 + tx;
    let p0y = b * w1 + d * h1 + ty;

    // left bottom
    let p1x = a * w0 + c * h1 + tx;
    let p1y = b * w0 + d * h1 + ty;

    // right top
    let p2x = a * w0 + c * h0 + tx;
    let p2y = b * w0 + d * h0 + ty;

    // right bottom
    let p3x = a * w1 + c * h0 + tx;
    let p3y = b * w1 + d * h0 + ty;

    [
        SpriteVertex {
            position: [p0x, p0y, 0.0],
            tex_coords: [x0, y0],
        },
        SpriteVertex {
            position: [p3x, p3y, 0.0],
            tex_coords: [x0, y1],
        },
        SpriteVertex {
            position: [p2x, p2y, 0.0],
            tex_coords: [x1, y1],
        },
        SpriteVertex {
            position: [p1x, p1y, 0.0],
            tex_coords: [x1, y0],
        },
    ]
}
