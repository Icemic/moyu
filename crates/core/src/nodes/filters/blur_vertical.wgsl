// Gaussian Blur - Vertical Pass
// 对输入纹理进行垂直方向的模糊

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct BlurParams {
    texel_size: vec2<f32>,  // 1.0 / texture_size
    blur_radius: f32,
    _padding: f32,
};

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

@group(0) @binding(2)
var<uniform> params: BlurParams;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    // 生成全屏四边形
    var uv: vec2<f32>;
    switch vertex_index {
        case 0u: { uv = vec2<f32>(0.0, 0.0); }
        case 1u: { uv = vec2<f32>(1.0, 0.0); }
        case 2u: { uv = vec2<f32>(0.0, 1.0); }
        case 3u: { uv = vec2<f32>(1.0, 0.0); }
        case 4u: { uv = vec2<f32>(1.0, 1.0); }
        default: { uv = vec2<f32>(0.0, 1.0); }
    }
    
    // NDC 坐标 (-1 to 1)
    let pos = uv * 2.0 - 1.0;
    output.position = vec4<f32>(pos.x, -pos.y, 0.0, 1.0);
    output.uv = uv;
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0);
    var total_weight = 0.0;
    
    // 计算 sigma（标准差）
    let sigma = params.blur_radius / 3.0;
    let sigma_squared = sigma * sigma;
    
    // 垂直方向采样
    let radius_int = i32(ceil(params.blur_radius));
    for (var y = -radius_int; y <= radius_int; y = y + 1) {
        let offset = vec2<f32>(0.0, f32(y) * params.texel_size.y);
        let sample_uv = input.uv + offset;
        
        // Gaussian 权重：exp(-(y^2) / (2 * sigma^2))
        let y_squared = f32(y) * f32(y);
        let weight = exp(-y_squared / (2.0 * sigma_squared));
        
        color += textureSample(source_texture, source_sampler, sample_uv) * weight;
        total_weight += weight;
    }
    
    return color / total_weight;
}
