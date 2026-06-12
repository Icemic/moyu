struct RenderUniform {
  position: vec2<f32>,
  size: vec2<f32>,
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
var<uniform> render_uniform: RenderUniform;

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
  let position = render_uniform.position + output.uv * render_uniform.size;
  output.position = mvp.mvp * vec4<f32>(position, 0.0, 1.0);
  return output;
}
