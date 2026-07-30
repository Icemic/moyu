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
  let color = sampleChannel0(input.uv);
  let scan_position = fract(builtins.time * speed * 0.2);
  let distance = abs(input.uv.y - scan_position);
  let band = smoothstep(0.08, 0.0, distance) * strength;
  let scan_color = color.rgb + vec3<f32>(0.15, 0.35, 0.5) * band;
  return vec4<f32>(scan_color, color.a);
}
