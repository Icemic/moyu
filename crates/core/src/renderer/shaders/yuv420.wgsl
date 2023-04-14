struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

@group(0) @binding(0)
var texture_y: texture_2d<f32>;
@group(0) @binding(1)
var sampler_y: sampler;

@group(0) @binding(2)
var texture_u: texture_2d<f32>;
@group(0) @binding(3)
var sampler_u: sampler;

@group(0) @binding(4)
var texture_v: texture_2d<f32>;
@group(0) @binding(5)
var sampler_v: sampler;

struct Uniforms {
    mode: i32,
}

@group(1) @binding(0)
var<uniform> uniforms: Uniforms;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_y = textureSample(texture_y, sampler_y, in.tex_coords).r - 0.0625;
    let tex_u = textureSample(texture_u, sampler_u, in.tex_coords).r - 0.5;

    var tex_v: f32;

    switch uniforms.mode {
        case 0: {
            tex_v = textureSample(texture_v, sampler_v, in.tex_coords).r - 0.5;
        }
        case 1: {
            tex_v = textureSample(texture_u, sampler_u, in.tex_coords).g - 0.5;
        }
        default: {
            tex_v = textureSample(texture_v, sampler_v, in.tex_coords).r - 0.5;
        }
    }

    //ITU BT.709 Default Matrix (Video Range)
    var rgb: vec3<f32> = mat3x3<f32>(1.164, 1.164, 1.164, 0.0, -0.213, 2.112, 1.793, -0.533, 0.0) * vec3<f32>(tex_y, tex_u, tex_v);

    return toLinear(vec4<f32>(rgb, 1.0));
}

// taken from https://gamedev.stackexchange.com/questions/92015/optimized-linear-to-srgb-glsl
fn fromLinear(linearRGB: vec4<f32>) -> vec4<f32> {
    let cutoff: vec3<f32> = step(linearRGB.rgb, vec3<f32>(0.0031308));
    let higher: vec3<f32> = vec3<f32>(1.055) * pow(linearRGB.rgb, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    let lower: vec3<f32> = linearRGB.rgb * vec3<f32>(12.92);

    return vec4<f32>(mix(higher, lower, cutoff), linearRGB.a);
}

fn toLinear(sRGB: vec4<f32>) -> vec4<f32> {
    let cutoff: vec3<f32> = step(sRGB.rgb, vec3<f32>(0.04045));
    let higher: vec3<f32> = pow((sRGB.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    let lower: vec3<f32> = sRGB.rgb / vec3<f32>(12.92);

    return vec4<f32>(mix(higher, lower, vec3<f32>(cutoff)), sRGB.a);
}