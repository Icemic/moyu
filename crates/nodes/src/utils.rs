use bytemuck::{Pod, Zeroable};

use moyu_core::base::*;
use moyu_core::traits::Node;

pub static QUAD_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];
pub static QUAD_INDICES_COUNT: u32 = QUAD_INDICES.len() as u32;

#[inline]
pub fn calculate_quad_vertices(
    node: &dyn Node,
    tex_width: f32,
    tex_height: f32,
    origin: &[f32; 2],
    area: &[f32; 4],
    // for nineslice stretch
    scale: &[f32; 2],
) -> [QuadVertex; 4] {
    let [x0, y0, x1, y1] = area.to_owned();
    let [x_scale, y_scale] = scale.to_owned();

    // scale size to fit area
    let width = tex_width * (x1 - x0);
    let height = tex_height * (y1 - y0);

    let global_transform = node.base().global_transform();

    let w0 = origin[0] * tex_width;
    let w1 = w0 + width * x_scale;
    let h0 = origin[1] * tex_height;
    let h1 = h0 + height * y_scale;

    let p0 = global_transform.transform_point3a(glam::vec3a(w0, h0, 0.0));
    let p1 = global_transform.transform_point3a(glam::vec3a(w1, h0, 0.0));
    let p2 = global_transform.transform_point3a(glam::vec3a(w1, h1, 0.0));
    let p3 = global_transform.transform_point3a(glam::vec3a(w0, h1, 0.0));

    let mut tint = node.base().tint().to_array();
    tint[3] *= node.base().global_opacity();

    [
        QuadVertex {
            position: [p0.x, p0.y, p0.z],
            tex_coords: [x0, y0],
            tint,
        },
        QuadVertex {
            position: [p3.x, p3.y, p3.z],
            tex_coords: [x0, y1],
            tint,
        },
        QuadVertex {
            position: [p2.x, p2.y, p2.z],
            tex_coords: [x1, y1],
            tint,
        },
        QuadVertex {
            position: [p1.x, p1.y, p1.z],
            tex_coords: [x1, y0],
            tint,
        },
    ]
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct QuadVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub tint: [f32; 4],
}

impl VertexDesc for QuadVertex {
    fn attribs() -> &'static [wgpu::VertexAttribute] {
        static SPRITE_ATTRIBS: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];

        &SPRITE_ATTRIBS
    }
}
