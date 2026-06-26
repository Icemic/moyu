struct RenderUniform {
    position: vec2<f32>,
    size: vec2<f32>,
}

struct HiddenUniform {
    max_uv: vec2<f32>,
    padding: vec2<f32>,
}

struct BuiltinsUniform {
    time: f32,
    time_delta: f32,
    progress: f32,
    effect_id: i32,
    frame: u32,
    channel_count: u32,
    stage_size: vec2<f32>,
}

struct ParamsUniform {
    slots: array<vec4<u32>, 8>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(1) @binding(0)
var<uniform> render_uniform: RenderUniform;

@group(1) @binding(1)
var<uniform> hidden_uniform: HiddenUniform;

@group(1) @binding(2)
var<uniform> builtins: BuiltinsUniform;

@group(1) @binding(3)
var<uniform> params: ParamsUniform;

@group(1) @binding(4)
var texture_sampler: sampler;

@group(1) @binding(5)
var channel0: texture_2d<f32>;

@group(1) @binding(6)
var channel1: texture_2d<f32>;

@group(1) @binding(7)
var channel2: texture_2d<f32>;

@group(1) @binding(8)
var channel3: texture_2d<f32>;

fn apply_crossfade(from_color: vec4<f32>, to_color: vec4<f32>, progress: f32) -> vec4<f32> {
    return mix(from_color, to_color, progress);
}

fn composite_over(top: vec4<f32>, bottom: vec4<f32>) -> vec4<f32> {
    return top + bottom * (1.0 - top.a);
}

fn uv_in_bounds(uv: vec2<f32>) -> bool {
    return all(uv >= vec2<f32>(0.0, 0.0)) && all(uv <= vec2<f32>(1.0, 1.0));
}

fn sample_texture(texture: texture_2d<f32>, uv: vec2<f32>) -> vec4<f32> {
    if !uv_in_bounds(uv) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    return textureSample(texture, texture_sampler, uv * hidden_uniform.max_uv);
}

fn read_directional_progress(direction: u32, uv: vec2<f32>) -> f32 {
    switch direction {
        case 0u: {
            return 1.0 - uv.x;
        }
        case 1u: {
            return uv.x;
        }
        case 2u: {
            return 1.0 - uv.y;
        }
        default: {
            return uv.y;
        }
    }
}

fn direction_offset(direction: u32, amount: f32) -> vec2<f32> {
    switch direction {
        case 0u: {
            return vec2<f32>(-amount, 0.0);
        }
        case 1u: {
            return vec2<f32>(amount, 0.0);
        }
        case 2u: {
            return vec2<f32>(0.0, -amount);
        }
        default: {
            return vec2<f32>(0.0, amount);
        }
    }
}

fn sample_translated(texture: texture_2d<f32>, uv: vec2<f32>, direction: u32, amount: f32) -> vec4<f32> {
    return sample_texture(texture, uv - direction_offset(direction, amount));
}

fn sample_scaled(texture: texture_2d<f32>, uv: vec2<f32>, scale: f32, origin: vec2<f32>) -> vec4<f32> {
    let safe_scale = max(scale, 0.0001);
    let scaled_uv = (uv - origin) / safe_scale + origin;
    return sample_texture(texture, scaled_uv);
}

fn sample_pixellated(texture: texture_2d<f32>, uv: vec2<f32>, exponent: f32) -> vec4<f32> {
    let dimensions = max(vec2<f32>(textureDimensions(texture)) * hidden_uniform.max_uv, vec2<f32>(1.0, 1.0));
    let max_exponent = ceil(log2(max(dimensions.x, dimensions.y)));
    let block_size = min(vec2<f32>(exp2(clamp(exponent, 0.0, max_exponent))) / dimensions, vec2<f32>(1.0, 1.0));
    let snapped_uv = clamp((floor(uv / block_size) + vec2<f32>(0.5, 0.5)) * block_size, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
    return sample_texture(texture, snapped_uv);
}

fn apply_wipe(from_color: vec4<f32>, to_color: vec4<f32>, progress: f32, uv: vec2<f32>) -> vec4<f32> {
    let softness = read_param_f32(0u);
    let direction = read_param_u32(1u);
    let edge_progress = read_directional_progress(direction, uv);

    if softness <= 0.0001 {
        let to_weight = select(0.0, 1.0, edge_progress <= progress);
        return mix(from_color, to_color, to_weight);
    }

    let to_weight = 1.0 - smoothstep(progress - softness, progress + softness, edge_progress);
    return mix(from_color, to_color, to_weight);
}

fn read_param_u32(index: u32) -> u32 {
    let lane = params.slots[index / 4u];
    switch index % 4u {
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
    if span <= 0.0001 {
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

    if progress < out_end {
        return mix(from_color, fade_color, fade_segment_progress(progress, 0.0, out_end));
    }

    if progress < hold_end {
        return fade_color;
    }

    return mix(fade_color, to_color, fade_segment_progress(progress, hold_end, in_end));
}

fn apply_push(progress: f32, uv: vec2<f32>) -> vec4<f32> {
    let direction = read_param_u32(0u);
    let from_color = sample_translated(channel0, uv, direction, progress);
    let to_color = sample_translated(channel1, uv, direction, progress - 1.0);
    return composite_over(from_color, to_color);
}

fn apply_slideaway(to_color: vec4<f32>, progress: f32, uv: vec2<f32>) -> vec4<f32> {
    let direction = read_param_u32(0u);
    let from_color = sample_translated(channel0, uv, direction, progress);
    return composite_over(from_color, to_color);
}

fn apply_zoom(from_color: vec4<f32>, progress: f32, uv: vec2<f32>) -> vec4<f32> {
    let start_scale = read_param_f32(0u);
    let end_scale = read_param_f32(1u);
    let stage_size = max(builtins.stage_size, vec2<f32>(1.0, 1.0));
    let rect_size = max(render_uniform.size, vec2<f32>(0.0001, 0.0001));
    let screen_origin = vec2<f32>(read_param_f32(2u), read_param_f32(3u));
    let origin = (screen_origin * stage_size - render_uniform.position) / rect_size;
    let scale = mix(start_scale, end_scale, progress);

    if start_scale <= end_scale {
        let to_color = sample_scaled(channel1, uv, scale, origin);
        return composite_over(to_color, from_color);
    }

    let scaled_from_color = sample_scaled(channel0, uv, scale, origin);
    return composite_over(scaled_from_color, sample_texture(channel1, uv));
}

fn pixellate_step_progress(progress: f32, steps: f32) -> f32 {
    return min(floor(progress * (steps + 1.0)), steps);
}

fn apply_pixellate(progress: f32, uv: vec2<f32>) -> vec4<f32> {
    let steps = f32(read_param_u32(0u));

    if progress < 0.5 {
        let phase = clamp(progress / 0.5, 0.0, 1.0);
        return sample_pixellated(channel0, uv, pixellate_step_progress(phase, steps));
    }

    let phase = clamp((progress - 0.5) / 0.5, 0.0, 1.0);
    return sample_pixellated(channel1, uv, pixellate_step_progress(1.0 - phase, steps));
}

fn read_mask_value(uv: vec2<f32>) -> f32 {
    let mask_sample = sample_texture(channel2, uv);
    return mask_sample.r;
}

fn apply_mask(from_color: vec4<f32>, to_color: vec4<f32>, progress: f32, uv: vec2<f32>) -> vec4<f32> {
    let softness = read_param_f32(0u);
    let reverse = read_param_u32(1u) != 0u;
    let raw_mask = read_mask_value(uv);
    let mask = select(raw_mask, 1.0 - raw_mask, reverse);

    if softness <= 0.0001 {
        let to_weight = select(0.0, 1.0, mask <= progress);
        return mix(from_color, to_color, to_weight);
    }

    let threshold = progress * (1.0 + softness * 2.0) - softness;
    let to_weight = smoothstep(mask - softness, mask + softness, threshold);
    return mix(from_color, to_color, to_weight);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let progress = builtins.progress;
    let from_color = sample_texture(channel0, input.uv);
    let to_color = sample_texture(channel1, input.uv);
    let effect_id = builtins.effect_id;

    switch effect_id {
        case 0: {
            return apply_crossfade(from_color, to_color, progress);
        }
        case 1: {
            return apply_wipe(from_color, to_color, progress, input.uv);
        }
        case 2: {
            return apply_fade(from_color, to_color, progress);
        }
        case 3: {
            return apply_push(progress, input.uv);
        }
        case 4: {
            return apply_slideaway(to_color, progress, input.uv);
        }
        case 5: {
            return apply_zoom(from_color, progress, input.uv);
        }
        case 6: {
            return apply_pixellate(progress, input.uv);
        }
        case 7: {
            return apply_mask(from_color, to_color, progress, input.uv);
        }
        default: {
            return from_color;
        }
    }
}
