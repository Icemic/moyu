struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) page: i32,
    @location(3) buffer: f32,
    @location(4) fill_buffer: f32,
    @location(5) gamma: f32,
    @location(6) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) @interpolate(flat) page: i32,
    @location(2) buffer: f32,
    @location(3) fill_buffer: f32,
    @location(4) gamma: f32,
    @location(5) color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> mvp_matrix: mat4x4<f32>;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = mvp_matrix * vec4<f32>(model.position, 1.0);
    out.page = model.page;
    out.buffer = model.buffer;
    out.fill_buffer = model.fill_buffer;
    out.gamma = model.gamma;
    out.color = model.color;
    return out;
}

// Fragment shader

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var samp: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = textureSample(texture, samp, in.tex_coords)[in.page];

    // from https://github.com/jinleili/sdf-text-view/blob/86ae02c83fd66b69be3c74493a93b73bf258c9ca/shader-wgsl/text.wgsl#L38
    let fill_gamma = length(vec2<f32>(dpdx(1. - dist), dpdy(1. - dist))) * 0.707107;

    let gamma = fill_gamma + in.gamma;

    let outer = smoothstep(in.buffer - gamma, in.buffer + gamma, dist);
    let inner = smoothstep(in.fill_buffer - fill_gamma, in.fill_buffer + fill_gamma, dist);
    return vec4(in.color.rgb, (outer - inner) * in.color.a);
}
