struct VertexInput {
    @location(0) position: vec2<f32>,
};

// Instance layout of the plain pipeline: everything a sprite needs without tiling.
struct InstanceInput {
    @location(2) transform_0: vec4<f32>,
    @location(3) transform_1: vec4<f32>,
    @location(4) transform_2: vec4<f32>,
    @location(5) transform_3: vec4<f32>,
    @location(6) local_bounds: vec4<f32>,
    @location(7) uv_bounds: vec4<f32>,
    @location(8) tint: vec4<f32>,
};

// Instance layout of the tiling pipeline: adds the repeat counts and mirror flag
// the fragment stage needs to tile the slice pattern.
struct TilingInstanceInput {
    @location(2) transform_0: vec4<f32>,
    @location(3) transform_1: vec4<f32>,
    @location(4) transform_2: vec4<f32>,
    @location(5) transform_3: vec4<f32>,
    @location(6) local_bounds: vec4<f32>,
    @location(7) uv_bounds: vec4<f32>,
    @location(8) tint: vec4<f32>,
    // (repeat_x, repeat_y, mirror, unused)
    @location(9) uv_params: vec4<f32>,
};

// The plain pipeline interpolates the final UV and tint only.
struct PlainVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

// The tiling pipeline carries the slice bounds and pattern coordinates so the
// fragment stage can repeat the pattern; only repeat and mirror use it.
struct TileVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv_bounds: vec4<f32>,
    // Position inside the slice measured in pattern repeats, interpolated per fragment.
    @location(1) pattern_coords: vec2<f32>,
    @location(2) uv_params: vec4<f32>,
    @location(3) tint: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> mvp_matrix: mat4x4<f32>;

fn world_position(
    position: vec2<f32>,
    local_bounds: vec4<f32>,
    transform_0: vec4<f32>,
    transform_1: vec4<f32>,
    transform_3: vec4<f32>,
) -> vec2<f32> {
    // Local position in pixels; local_bounds: (left, top, right, bottom).
    let local_x = mix(local_bounds.x, local_bounds.z, position.x);
    let local_y = mix(local_bounds.y, local_bounds.w, position.y);

    // Construct global transform matrix from columns.
    let a = transform_0.x;
    let b = transform_0.y;
    let c = transform_1.x;
    let d = transform_1.y;
    let tx = transform_3.x;
    let ty = transform_3.y;

    return vec2<f32>(a * local_x + c * local_y + tx, b * local_x + d * local_y + ty);
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> PlainVertexOutput {
    var out: PlainVertexOutput;

    let world = world_position(
        model.position,
        instance.local_bounds,
        instance.transform_0,
        instance.transform_1,
        instance.transform_3,
    );
    out.clip_position = mvp_matrix * vec4<f32>(world, 0.0, 1.0);

    out.tex_coords = mix(instance.uv_bounds.xy, instance.uv_bounds.zw, model.position.xy);
    out.tint = instance.tint;
    return out;
}

@vertex
fn vs_tile(
    model: VertexInput,
    instance: TilingInstanceInput,
) -> TileVertexOutput {
    var out: TileVertexOutput;

    let world = world_position(
        model.position,
        instance.local_bounds,
        instance.transform_0,
        instance.transform_1,
        instance.transform_3,
    );
    out.clip_position = mvp_matrix * vec4<f32>(world, 0.0, 1.0);

    out.uv_bounds = instance.uv_bounds;
    out.pattern_coords = model.position.xy * instance.uv_params.xy;
    out.uv_params = instance.uv_params;
    out.tint = instance.tint;
    return out;
}

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var samp: sampler;

@fragment
fn fs_main(in: PlainVertexOutput) -> @location(0) vec4<f32> {
    let t = textureSample(texture, samp, in.tex_coords);
    return vec4(t.rgb * in.tint.rgb * in.tint.a, t.a * in.tint.a);
}

@fragment
fn fs_tile(in: TileVertexOutput) -> @location(0) vec4<f32> {
    // Repeat the slice pattern inside its bounds. With a repeat count of 1 the
    // fraction is the interpolated position itself, so the callers that do not tile
    // never reach this entry point.
    let cell = floor(in.pattern_coords);
    let cell_fraction = in.pattern_coords - cell;

    // Mirror flips every odd cell; the parity term keeps the flip amount at 0 or 1.
    let parity = cell - 2.0 * floor(cell * 0.5);
    let flip = parity * in.uv_params.z;
    let pattern = mix(cell_fraction, vec2<f32>(1.0) - cell_fraction, flip);

    let uv = mix(in.uv_bounds.xy, in.uv_bounds.zw, pattern);

    // The gradient follows the untiled coordinate, so mip levels stay consistent
    // across repeat seams instead of jumping at each pattern boundary.
    let uv_linear = mix(in.uv_bounds.xy, in.uv_bounds.zw, in.pattern_coords);
    let t = textureSampleGrad(texture, samp, uv, dpdx(uv_linear), dpdy(uv_linear));

    return vec4(t.rgb * in.tint.rgb * in.tint.a, t.a * in.tint.a);
}
