struct BuiltinsUniform {
  time: f32,
  time_delta: f32,
  progress: f32,
  effect_id: i32,
  frame: u32,
  channel_count: u32,
  _padding1: vec2<u32>,
}

struct ParamsUniform {
  slots: array<vec4<u32>, 8>,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@group(1) @binding(1)
var<uniform> builtins: BuiltinsUniform;

@group(1) @binding(2)
var<uniform> params: ParamsUniform;

@group(1) @binding(3)
var texture_sampler: sampler;

@group(1) @binding(4)
var channel0: texture_2d<f32>;

@group(1) @binding(5)
var channel1: texture_2d<f32>;

fn apply_crossfade(from_color: vec4<f32>, to_color: vec4<f32>, progress: f32) -> vec4<f32> {
  return mix(from_color, to_color, progress);
}

fn apply_wipe(from_color: vec4<f32>, to_color: vec4<f32>, progress: f32, wipe_y: f32) -> vec4<f32> {
  let feather = 0.02;
  let to_weight = 1.0 - smoothstep(progress - feather, progress + feather, wipe_y);
  return mix(from_color, to_color, to_weight);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let progress = builtins.progress;
  let from_color = textureSample(channel0, texture_sampler, input.uv);
  let to_color = textureSample(channel1, texture_sampler, input.uv);
  let effect_id = builtins.effect_id;

  switch (effect_id) {
    case 0: {
      return apply_crossfade(from_color, to_color, progress);
    }
    case 1: {
      return apply_wipe(from_color, to_color, progress, input.uv.y);
    }
    default: {
      return from_color;
    }
  }
}
