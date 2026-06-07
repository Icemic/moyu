struct TransitionParams {
  position: vec2<f32>,
  size: vec2<f32>,
  uv_scale: vec2<f32>,
  progress: f32,
  effect_id: i32,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

struct MVPMatrix {
  mvp: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> mvp: MVPMatrix;

@group(1) @binding(0)
var<uniform> params: TransitionParams;

@group(1) @binding(1)
var texture_sampler: sampler;

@group(1) @binding(2)
var from_texture: texture_2d<f32>;

@group(1) @binding(3)
var to_texture: texture_2d<f32>;

fn apply_crossfade(from_color: vec4<f32>, to_color: vec4<f32>, progress: f32) -> vec4<f32> {
  return mix(from_color, to_color, progress);
}

fn apply_wipe(from_color: vec4<f32>, to_color: vec4<f32>, progress: f32, wipe_y: f32) -> vec4<f32> {
  let feather = 0.02;
  let to_weight = 1.0 - smoothstep(progress - feather, progress + feather, wipe_y);
  return mix(from_color, to_color, to_weight);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
  var uvs = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(0.0, 1.0),
  );

  var output: VertexOutput;
  output.uv = uvs[vertex_index];
  let position = params.position + output.uv * params.size;
  output.position = mvp.mvp * vec4<f32>(position, 0.0, 1.0);
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let progress = clamp(params.progress, 0.0, 1.0);
  let uv = input.uv * params.uv_scale;
  let from_color = textureSample(from_texture, texture_sampler, uv);

  if (params.effect_id < 0) {
    return from_color;
  }

  let to_color = textureSample(to_texture, texture_sampler, uv);
  switch (params.effect_id) {
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
