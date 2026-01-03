// Color Adjust Shader - Brightness, Contrast, Saturation

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    var uv: vec2<f32>;
    switch vertex_index {
        case 0u: { uv = vec2<f32>(0.0, 0.0); }
        case 1u: { uv = vec2<f32>(1.0, 0.0); }
        case 2u: { uv = vec2<f32>(0.0, 1.0); }
        case 3u: { uv = vec2<f32>(1.0, 0.0); }
        case 4u: { uv = vec2<f32>(1.0, 1.0); }
        default: { uv = vec2<f32>(0.0, 1.0); }
    }
    output.position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    output.position.y = -output.position.y; // Flip Y for NDC
    output.uv = uv;
    return output;
}

struct ColorAdjustParams {
    brightness: f32,
    contrast: f32,
    saturation: f32,
    _padding: f32,
};

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> params: ColorAdjustParams;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(source_texture, source_sampler, input.uv);
    var rgb = color.rgb;
    
    // Brightness
    rgb = rgb * params.brightness;
    
    // Contrast
    rgb = (rgb - 0.5) * params.contrast + 0.5;
    
    // Saturation
    let gray = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
    rgb = mix(vec3<f32>(gray), rgb, params.saturation);
    
    return vec4<f32>(rgb, color.a);
}
