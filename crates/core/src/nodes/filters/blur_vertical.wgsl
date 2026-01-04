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
    let inv_two_sigma_sq = 1.0 / (2.0 * sigma_squared);

    // 1. 中心采样
    color += textureSample(source_texture, source_sampler, input.uv);
    total_weight += 1.0;

    // 2. 使用线性采样优化 (Linear Sampling Optimization)
    let r = params.blur_radius;
    for (var i = 1.0; i <= r; i += 2.0) {
        let w1 = exp(-(i * i) * inv_two_sigma_sq);
        let w2 = exp(-((i + 1.0) * (i + 1.0)) * inv_two_sigma_sq);
        
        let m_weight = w1 + w2;
        let m_offset = (i * w1 + (i + 1.0) * w2) / m_weight;
        
        let offset_vec = vec2<f32>(0.0, m_offset * params.texel_size.y);
        color += textureSample(source_texture, source_sampler, input.uv + offset_vec) * m_weight;
        color += textureSample(source_texture, source_sampler, input.uv - offset_vec) * m_weight;
        total_weight += m_weight * 2.0;
    }
    
    return color / total_weight;
}
