use csscolorparser::Color;

#[inline]
pub fn tint_to_vec4(tint: &Color, alpha: f32) -> [f32; 4] {
    [
        tint.r as f32,
        tint.g as f32,
        tint.b as f32,
        tint.a as f32 * alpha,
    ]
}
