use crate::core::render_command::FilterKind;
use crate::core::texture_pool::TexturePool;
use wgpu::*;

/// 滤镜渲染器 Trait
pub trait FilterRenderer {
    /// 滤镜名称
    fn name(&self) -> &'static str;

    /// 执行滤镜
    /// input: 输入纹理视图
    /// output: 输出纹理视图
    /// filter: 滤镜配置
    fn execute(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        input: &TextureView,
        output: &TextureView,
        filter: &FilterKind,
        width: u32,
        height: u32,
        scale: f32,
        pool: &mut TexturePool,
        timestamp: f64,
    );

    /// Reset frame-local state (called at the beginning of each frame)
    fn reset_frame(&mut self);
}
