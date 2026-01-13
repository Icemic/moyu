struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    var uv: vec2<f32>;
    switch vertex_index {
        case 0u: { uv = vec2<f32>(0.0, 0.0); }
        case 1u: { uv = vec2<f32>(1.0, 0.0); }
        case 2u: { uv = vec2<f32>(0.0, 1.0); }
        case 3u: { uv = vec2<f32>(1.0, 0.0); }
        case 4u: { uv = vec2<f32>(1.0, 1.0); }
        default: { uv = vec2<f32>(0.0, 1.0); }
    }
    let pos = uv * 2.0 - 1.0;
    output.position = vec4<f32>(pos.x, -pos.y, 0.0, 1.0);
    output.uv = uv;
    return output;
}

@group(0) @binding(0)
var t_source: texture_2d<f32>;
@group(0) @binding(1)
var s_source: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_source, s_source, input.uv);
}

struct BlendParams {
    weight: f32,
    _padding1: f32,
    _padding2: f32,
    _padding3: f32,
};

@group(1) @binding(0)
var t_source2: texture_2d<f32>;
@group(1) @binding(1)
var<uniform> blend_params: BlendParams;

@fragment
fn fs_blend(input: VertexOutput) -> @location(0) vec4<f32> {
    let color1 = textureSample(t_source, s_source, input.uv);
    let color2 = textureSample(t_source2, s_source, input.uv);
    return mix(color1, color2, blend_params.weight);
}
