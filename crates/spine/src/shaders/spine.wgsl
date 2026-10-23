// Spine draw pass.
//
// Vertices arrive in world units. `spine.node` shifts them onto the node by the
// reference bounds origin and moves them into stage units; the engine MVP takes
// them into clip space. Nothing is flipped: the runtime's `Bone::yDown` default
// already makes its world space grow downwards like the node's.
//
// The node matrix is uploaded in the contract's row-major layout and applied as
// `vector * matrix`. A row-major matrix times a row vector is the same product as
// its transpose times that vector as a column, so this is the usual transform
// with the transpose moved from the CPU into the shader: `mvp.matrix` comes from
// the engine in the column-major layout it applies as `matrix * vec`, and is the
// only one multiplied on the left.
//
// Culling stays off: a mirrored or negative-scale node reverses the winding of
// its own geometry.

struct Mvp {
    matrix: mat4x4<f32>,
};

struct Spine {
    // World units to stage units; row-major, applied as `v * node`.
    node: mat4x4<f32>,
    // Node tint (rgb) and global opacity (a).
    base: vec4<f32>,
};

@group(0) @binding(0) var<uniform> mvp: Mvp;
@group(1) @binding(0) var<uniform> spine: Spine;
@group(2) @binding(0) var page: texture_2d<f32>;
@group(2) @binding(1) var page_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // Unpacked and normalized, still premultiplied.
    @location(1) light: vec4<f32>,
    // Full four-component dark color: the formula below uses its alpha too.
    @location(2) dark: vec4<f32>,
};

// The contract packs colors as `b | g << 8 | r << 16 | a << 24`, which reaches
// the shader as one byte per channel in `b, g, r, a` order; the channels are put
// back in place here, then read as normalized floats by the vertex format.
fn unpack(color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(color.z, color.y, color.x, color.w);
}

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) light: vec4<f32>,
    @location(3) dark: vec4<f32>,
) -> VertexOut {
    var out: VertexOut;
    let world = vec4<f32>(position, 0.0, 1.0);
    out.position = mvp.matrix * (world * spine.node);
    out.uv = uv;
    out.light = unpack(light);
    out.dark = unpack(dark);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // Both the texture and the vertex colors arrive premultiplied, and every
    // step below stays in premultiplied space: the node tint and opacity scale
    // rgb and alpha together.
    let sampled = textureSample(page, page_sampler, in.uv);

    // Two-color tint, applied to the sampled color before the vertex colors.
    // `dark` is a complete vec4, so its alpha takes part; it is 1 for every
    // vertex today, but the formula is kept general on purpose.
    let dark_mix = (sampled.a - 1.0) * in.dark.a + 1.0;
    let tinted = (dark_mix - sampled.rgb) * in.dark.rgb + sampled.rgb * in.light.rgb;

    let scale = in.light.a * spine.base.a;
    return vec4<f32>(tinted * spine.base.rgb * scale, sampled.a * scale);
}
