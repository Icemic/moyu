// Blit Shader - 将纹理绘制到全屏四边形
// 用于 Backdrop 等效果

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// MVP 矩阵（用于将屏幕坐标转换为裁剪空间）
struct MVPMatrix {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> mvp: MVPMatrix;

// 绘制参数（位置和大小）
struct BlitParams {
    position: vec2<f32>,
    size: vec2<f32>,
    tint: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> params: BlitParams;

@group(1) @binding(1)
var texture_sampler: sampler;

@group(1) @binding(2)
var texture_view: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    // 生成全屏四边形的顶点
    var uv: vec2<f32>;
    switch vertex_index {
        case 0u: { uv = vec2<f32>(0.0, 0.0); }
        case 1u: { uv = vec2<f32>(1.0, 0.0); }
        case 2u: { uv = vec2<f32>(0.0, 1.0); }
        case 3u: { uv = vec2<f32>(1.0, 0.0); }
        case 4u: { uv = vec2<f32>(1.0, 1.0); }
        default: { uv = vec2<f32>(0.0, 1.0); }
    }
    
    // 根据参数计算顶点位置
    let pos = params.position + uv * params.size;
    
    // 应用 MVP 变换
    output.position = mvp.mvp * vec4<f32>(pos, 0.0, 1.0);
    output.uv = uv;
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(texture_view, texture_sampler, input.uv);
    return vec4<f32>(color.rgb * params.tint.rgb * params.tint.a, color.a * params.tint.a);
}
