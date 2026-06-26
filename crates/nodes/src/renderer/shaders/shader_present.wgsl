struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct HiddenUniform {
    max_uv: vec2<f32>,
    padding: vec2<f32>,
}

@group(1) @binding(1)
var<uniform> hidden_uniform: HiddenUniform;

@group(1) @binding(2)
var texture_sampler: sampler;

@group(1) @binding(3)
var source_texture: texture_2d<f32>;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(source_texture, texture_sampler, input.uv * hidden_uniform.max_uv);
}
