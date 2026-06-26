struct RenderUniform {
    position: vec2<f32>,
    size: vec2<f32>,
}

struct HiddenUniform {
    max_uv: vec2<f32>,
    padding: vec2<f32>,
}

struct BuiltinsUniform {
    time: f32,
    time_delta: f32,
    progress: f32,
    effect_id: i32,
    frame: u32,
    channel_count: u32,
    stage_size: vec2<f32>,
}

@group(1) @binding(0)
var<uniform> render_uniform: RenderUniform;

@group(1) @binding(1)
var<uniform> moyu_hidden_uniform_internal: HiddenUniform;

@group(1) @binding(2)
var<uniform> builtins: BuiltinsUniform;

@group(1) @binding(4)
var texture_sampler: sampler;

@group(1) @binding(5)
var channel0: texture_2d<f32>;

@group(1) @binding(6)
var channel1: texture_2d<f32>;

@group(1) @binding(7)
var channel2: texture_2d<f32>;

@group(1) @binding(8)
var channel3: texture_2d<f32>;

fn moyu_sample_uv_internal(uv: vec2<f32>) -> vec2<f32> {
    return uv * moyu_hidden_uniform_internal.max_uv;
}

fn sampleChannel0(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(channel0, texture_sampler, moyu_sample_uv_internal(uv));
}

fn sampleChannel1(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(channel1, texture_sampler, moyu_sample_uv_internal(uv));
}

fn sampleChannel2(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(channel2, texture_sampler, moyu_sample_uv_internal(uv));
}

fn sampleChannel3(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(channel3, texture_sampler, moyu_sample_uv_internal(uv));
}
