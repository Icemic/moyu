struct ParamsUniform {
  slots: array<vec4<u32>, 8>,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@group(1) @binding(3)
var<uniform> params: ParamsUniform;

fn read_param_u32(index: u32) -> u32 {
  let lane = params.slots[index / 4u];
  switch (index % 4u) {
    case 0u: { return lane.x; }
    case 1u: { return lane.y; }
    case 2u: { return lane.z; }
    default: { return lane.w; }
  }
}

fn read_param_f32(index: u32) -> f32 {
  return bitcast<f32>(read_param_u32(index));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let strength = read_param_f32(0u);
  let speed = read_param_f32(1u);
  let offset = sin(builtins.time * speed) * 0.02 * strength;
  let center = sampleChannel0(input.uv);
  let red = sampleChannel0(vec2<f32>(input.uv.x + offset, input.uv.y)).r;
  let blue = sampleChannel0(vec2<f32>(input.uv.x - offset, input.uv.y)).b;
  return vec4<f32>(red, center.g, blue, center.a);
}
