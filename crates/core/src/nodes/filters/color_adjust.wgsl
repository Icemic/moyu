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
    amount: f32,
    mode: u32,
    _padding: vec2<f32>,
};

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> params: ColorAdjustParams;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(source_texture, source_sampler, input.uv);

    // Avoid division by zero
    if (color.a <= 0.0) {
        return vec4<f32>(0.0);
    }

    // Unpremultiply alpha
    var rgb = color.rgb / color.a;
    
    // Filter logic
    switch params.mode {
        case 0u: { // Brightness
            rgb = rgb * params.amount;
        }
        case 1u: { // Contrast
            rgb = (rgb - 0.5) * params.amount + 0.5;
        }
        case 2u: { // Saturation
            let gray = dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            rgb = mix(vec3<f32>(gray), rgb, params.amount);
        }
        case 3u: { // Hue Rotate
            let angle = params.amount * 3.14159265 / 180.0;
            let cosV = cos(angle);
            let sinV = sin(angle);
            
            let r = vec3<f32>(
                0.213 + cosV * 0.787 - sinV * 0.213,
                0.715 - cosV * 0.715 - sinV * 0.715,
                0.072 - cosV * 0.072 + sinV * 0.928
            );
            let g = vec3<f32>(
                0.213 - cosV * 0.213 + sinV * 0.143,
                0.715 + cosV * 0.285 + sinV * 0.140,
                0.072 - cosV * 0.072 - sinV * 0.283
            );
            let b = vec3<f32>(
                0.213 - cosV * 0.213 - sinV * 0.787,
                0.715 - cosV * 0.715 + sinV * 0.715,
                0.072 + cosV * 0.928 + sinV * 0.072
            );
            
            rgb = vec3<f32>(dot(rgb, r), dot(rgb, g), dot(rgb, b));
        }
        case 4u: { // Grayscale
            let gray = dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            rgb = mix(rgb, vec3<f32>(gray), saturate(params.amount));
        }
        case 5u: { // Sepia
            let r_sepia = dot(rgb, vec3<f32>(0.393, 0.769, 0.189));
            let g_sepia = dot(rgb, vec3<f32>(0.349, 0.686, 0.168));
            let b_sepia = dot(rgb, vec3<f32>(0.272, 0.534, 0.131));
            rgb = mix(rgb, vec3<f32>(r_sepia, g_sepia, b_sepia), saturate(params.amount));
        }
        case 6u: { // Invert
            rgb = mix(rgb, 1.0 - rgb, saturate(params.amount));
        }
        default: {}
    }
    
    // Re-premultiply and saturate
    return vec4<f32>(saturate(rgb) * color.a, color.a);
}
