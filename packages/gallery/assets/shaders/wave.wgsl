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
  let wave = sin((input.uv.y * 9.0 + builtins.time * speed) * 6.28318) * 0.015 * strength;
  let color = sampleChannel0(vec2<f32>(input.uv.x + wave, input.uv.y));
  let pulse = 0.75 + 0.25 * sin(builtins.time * speed * 3.0 + input.uv.x * 8.0);
  let shifted = vec3<f32>(color.r * pulse, color.g, color.b * (1.1 - pulse * 0.25));
  return vec4<f32>(mix(color.rgb, shifted, strength), color.a);
}
