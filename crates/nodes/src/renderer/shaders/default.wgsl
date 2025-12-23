struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct InstanceInput {
    @location(2) transform_0: vec4<f32>,
    @location(3) transform_1: vec4<f32>,
    @location(4) transform_2: vec4<f32>,
    @location(5) transform_3: vec4<f32>,
    @location(6) local_bounds: vec4<f32>,
    @location(7) uv_bounds: vec4<f32>,
    @location(8) tint: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> mvp_matrix: mat4x4<f32>;

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Calculate local position in pixels
    // local_bounds: (left, top, right, bottom)
    let local_x = mix(instance.local_bounds.x, instance.local_bounds.z, model.position.x);
    let local_y = mix(instance.local_bounds.y, instance.local_bounds.w, model.position.y);

    // Construct global transform matrix from columns
    let a = instance.transform_0.x;
    let b = instance.transform_0.y;
    let c = instance.transform_1.x;
    let d = instance.transform_1.y;
    let tx = instance.transform_2.x;
    let ty = instance.transform_2.y;

    let world_x = a * local_x + c * local_y + tx;
    let world_y = b * local_x + d * local_y + ty;

    out.clip_position = mvp_matrix * vec4<f32>(world_x, world_y, 0.0, 1.0);

    // Calculate UV
    // uv_bounds: (u0, v0, u1, v1)
    let uv_x = mix(instance.uv_bounds.x, instance.uv_bounds.z, model.position.x);
    let uv_y = mix(instance.uv_bounds.y, instance.uv_bounds.w, model.position.y);
    out.tex_coords = vec2<f32>(uv_x, uv_y);

    out.tint = instance.tint;
    return out;
}

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var samp: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = textureSample(texture, samp, in.tex_coords);
    return vec4(t.rgb * in.tint.rgb * in.tint.a, t.a * in.tint.a);
}
