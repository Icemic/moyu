use crate::base::*;
use crate::traits::Node;

#[inline]
pub fn calculate_rect_vertices(
    node: &dyn Node,
    tex_width: f64,
    tex_height: f64,
    area: &[f64; 4],
) -> [SpriteVertex; 4] {
    let [x0, y0, x1, y1] = area.to_owned();

    // scale size to fit area
    let width = tex_width * (x1 - x0);
    let height = tex_height * (y1 - y0);

    let global_transform = node.base().global_transform();

    let a = global_transform.a;
    let b = global_transform.b;
    let c = global_transform.c;
    let d = global_transform.d;
    let tx = global_transform.tx;
    let ty = 1. - global_transform.ty;

    let anchor = node.base().anchor();

    let w1 = -anchor.x * width;
    let w0 = w1 + width;
    let h1 = (-1. + anchor.y) * height;
    let h0 = h1 + height;

    // left top
    let p0x = a * w1 + c * h1 + tx - 1.;
    let p0y = b * w1 + d * h1 + ty;

    // left bottom
    let p1x = a * w0 + c * h1 + tx - 1.;
    let p1y = b * w0 + d * h1 + ty;

    // right top
    let p2x = a * w0 + c * h0 + tx - 1.;
    let p2y = b * w0 + d * h0 + ty;

    // right bottom
    let p3x = a * w1 + c * h0 + tx - 1.;
    let p3y = b * w1 + d * h0 + ty;

    [
        SpriteVertex {
            position: [p0x as f32, p0y as f32, 0.0],
            tex_coords: [x0 as f32, y1 as f32],
        },
        SpriteVertex {
            position: [p1x as f32, p1y as f32, 0.0],
            tex_coords: [x1 as f32, y1 as f32],
        },
        SpriteVertex {
            position: [p2x as f32, p2y as f32, 0.0],
            tex_coords: [x1 as f32, y0 as f32],
        },
        SpriteVertex {
            position: [p3x as f32, p3y as f32, 0.0],
            tex_coords: [x0 as f32, y0 as f32],
        },
    ]
}
