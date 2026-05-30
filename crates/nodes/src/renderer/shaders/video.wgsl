// YUV to RGB conversion shader for video playback.
// Supports I420 (3-plane YUV) and NV12 (Y + interleaved UV) formats.
// Uses BT.709 coefficients for HD video.

// Format constants
const FORMAT_I420: u32 = 0u;
const FORMAT_NV12: u32 = 1u;
const FORMAT_RGBA: u32 = 2u;
const FORMAT_BGRA: u32 = 3u;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) tint: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

struct VideoParams {
    @size(16) format: u32,
};

@group(0) @binding(0)
var<uniform> mvp_matrix: mat4x4<f32>;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = mvp_matrix * vec4<f32>(model.position, 1.0);
    out.tint = model.tint;
    return out;
}

@group(1) @binding(0)
var tex_y: texture_2d<f32>;
@group(1) @binding(1)
var tex_u: texture_2d<f32>;  // I420: R8 U plane; NV12: RG8 interleaved UV plane
@group(1) @binding(2)
var tex_v: texture_2d<f32>;  // I420: R8 V plane; NV12: unused (dummy)
@group(1) @binding(3)
var samp: sampler;
@group(1) @binding(4)
var<uniform> params: VideoParams;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (params.format == FORMAT_RGBA || params.format == FORMAT_BGRA) {
        let packed = textureSample(tex_y, samp, in.tex_coords);
        let rgb = select(packed.bgr, packed.rgb, params.format == FORMAT_RGBA);
        let alpha = packed.a * in.tint.a;
        return vec4<f32>(rgb * in.tint.rgb * alpha, alpha);
    }

    let y = textureSample(tex_y, samp, in.tex_coords).r;

    var u: f32;
    var v: f32;

    if (params.format == FORMAT_NV12) {
        // NV12: UV interleaved in tex_u as RG8Unorm
        let uv = textureSample(tex_u, samp, in.tex_coords);
        u = uv.r;
        v = uv.g;
    } else {
        // I420: separate U and V planes
        u = textureSample(tex_u, samp, in.tex_coords).r;
        v = textureSample(tex_v, samp, in.tex_coords).r;
    }

    // BT.709 YUV to RGB conversion
    // Y is in [0, 1], U and V are in [0, 1] (need to shift to [-0.5, 0.5])
    let u_adj = u - 0.5;
    let v_adj = v - 0.5;

    var rgb: vec3<f32>;
    rgb.r = y + 1.5748 * v_adj;
    rgb.g = y - 0.1873 * u_adj - 0.4681 * v_adj;
    rgb.b = y + 1.8556 * u_adj;

    let color = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));

    // Apply tint with premultiplied alpha
    return vec4<f32>(color * in.tint.rgb * in.tint.a, in.tint.a);
}
