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

fn read_param_u32(index: u32) -> u32 {
  let lane = params.slots[index / 4u];
  switch (index % 4u) {
    case 0u: {
      return lane.x;
    }
    case 1u: {
      return lane.y;
    }
    case 2u: {
      return lane.z;
    }
    default: {
      return lane.w;
    }
  }
}

fn read_param_f32(index: u32) -> f32 {
  return bitcast<f32>(read_param_u32(index));
}

fn fade_segment_progress(progress: f32, start: f32, end: f32) -> f32 {
  let span = end - start;
  if (span <= 0.0001) {
    return 1.0;
  }

  return clamp((progress - start) / span, 0.0, 1.0);
}

fn apply_fade(from_color: vec4<f32>, to_color: vec4<f32>, progress: f32) -> vec4<f32> {
  let out_ratio = read_param_f32(0u);
  let hold_ratio = read_param_f32(1u);
  let in_ratio = read_param_f32(2u);
  let fade_color = vec4<f32>(
    read_param_f32(3u),
    read_param_f32(4u),
    read_param_f32(5u),
    read_param_f32(6u),
  );
  let out_end = out_ratio;
  let hold_end = out_ratio + hold_ratio;
  let in_end = out_ratio + hold_ratio + in_ratio;

  if (progress < out_end) {
    return mix(from_color, fade_color, fade_segment_progress(progress, 0.0, out_end));
  }

  if (progress < hold_end) {
    return fade_color;
  }

  return mix(fade_color, to_color, fade_segment_progress(progress, hold_end, in_end));
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
    case 2: {
      return apply_fade(from_color, to_color, progress);
    }
    default: {
      return from_color;
    }
  }
}
