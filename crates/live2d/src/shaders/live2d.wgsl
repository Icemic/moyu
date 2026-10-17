// Live2D draw and mask passes.
//
// Snapshot data lives in canvas pixel space (top-left origin, y down). The
// drawable pass maps it through the node transform into stage units and then
// through the engine MVP into clip space; the mask pass maps it straight into
// mask clip space, which the host scales onto the mask texture's share of the
// render target (see `mask_correction` in the renderer).
//
// The snapshot matrices are uploaded in the contract's row-major layout and
// applied as `vector * matrix`. A row-major matrix times a row vector is the same
// product as its transpose times that vector as a column, so this is the usual
// transform with the transpose moved from the CPU into the shader: `mvp.matrix`
// comes from the engine in the column-major layout it applies as `matrix * vec`,
// and is the only one multiplied on the left.

struct Mvp {
    matrix: mat4x4<f32>,
};

struct Drawable {
    // Canvas pixels to stage units, row-major, applied as `v * node`.
    node: mat4x4<f32>,
    // Canvas pixels to mask sampling UV, row-major, applied as `v * clip`.
    clip: mat4x4<f32>,
    multiply: vec4<f32>,
    screen: vec4<f32>,
    base: vec4<f32>,
    channel: vec4<f32>,
    // x: 0 no mask, 1 normal, 2 inverted.
    mask_mode: vec4<f32>,
};

struct Mask {
    // Canvas pixels to mask clip space, already scaled onto the mask texture;
    // row-major, applied as `v * matrix`.
    matrix: mat4x4<f32>,
    // `[x_min, y_min, x_max, y_max]` in that same clip space.
    clip_rect: vec4<f32>,
    // One-hot channel this command writes.
    channel: vec4<f32>,
};

@group(0) @binding(0) var<uniform> mvp: Mvp;
@group(1) @binding(0) var<uniform> drawable: Drawable;
// The mask pass has its own group 1 layout; a second binding keeps both
// structures in one module.
@group(1) @binding(1) var<uniform> mask: Mask;
@group(2) @binding(0) var texture0: texture_2d<f32>;
@group(2) @binding(1) var sampler0: sampler;
@group(3) @binding(0) var texture1: texture_2d<f32>;
@group(3) @binding(1) var sampler1: sampler;

struct DrawOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) mask_uv: vec2<f32>,
};

@vertex
fn vs_draw(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>) -> DrawOut {
    var out: DrawOut;
    let canvas = vec4<f32>(position, 0.0, 1.0);
    out.position = mvp.matrix * (canvas * drawable.node);
    out.uv = uv;
    out.mask_uv = (canvas * drawable.clip).xy;
    return out;
}

@fragment
fn fs_draw(in: DrawOut) -> @location(0) vec4<f32> {
    // Model textures arrive premultiplied, so every step below stays in
    // premultiplied space: the multiply color scales the premultiplied rgb, the
    // screen color carries the sampled alpha, and the tint, opacity and mask
    // coefficient scale rgb and alpha together.
    let sampled = textureSample(texture0, sampler0, in.uv);
    let multiplied = sampled.rgb * drawable.multiply.rgb;
    let screened = multiplied + drawable.screen.rgb * sampled.a
        - multiplied * drawable.screen.rgb;

    var coefficient = 1.0;
    if (drawable.mask_mode.x > 0.5) {
        let value = dot(textureSample(texture1, sampler1, in.mask_uv), drawable.channel);
        if (drawable.mask_mode.x > 1.5) {
            coefficient = value;
        } else {
            coefficient = 1.0 - value;
        }
    }

    let scale = drawable.base.a * coefficient;
    return vec4<f32>(
        screened * drawable.base.rgb * scale,
        sampled.a * scale,
    );
}

struct MaskOut {
    @builtin(position) position: vec4<f32>,
    // Interpolated clip space position: the fragment shader needs it for the
    // clip rectangle test, and `@builtin(position)` holds pixels there.
    @location(0) clip_position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_mask(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>) -> MaskOut {
    var out: MaskOut;
    let clip = vec4<f32>(position, 0.0, 1.0) * mask.matrix;
    out.position = clip;
    out.clip_position = clip.xy;
    out.uv = uv;
    return out;
}

@fragment
fn fs_mask(in: MaskOut) -> @location(0) vec4<f32> {
    let is_inside = step(mask.clip_rect.x, in.clip_position.x)
        * step(mask.clip_rect.y, in.clip_position.y)
        * step(in.clip_position.x, mask.clip_rect.z)
        * step(in.clip_position.y, mask.clip_rect.w);
    return mask.channel * textureSample(texture0, sampler0, in.uv).a * is_inside;
}

struct FillOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_fill(@location(0) position: vec2<f32>) -> FillOut {
    var out: FillOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    return out;
}

@fragment
fn fs_fill() -> @location(0) vec4<f32> {
    // 1 means "not covered" for every mask channel.
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
