use serde::{Deserialize, Serialize};
use wgpu::{BindGroup, Buffer, RenderPipeline, TextureView};

use crate::base::Rect;

/// 滤镜配置（可序列化，用于 JS 互操作）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum FilterKind {
    Blur {
        radius: f32,
    },
    BlurFast {
        radius: f32,
        continuous: Option<bool>,
    },
    Brightness {
        amount: f32, // 0.0 = 全黑, 1.0 = 正常, 2.0 = 双倍亮度
    },
    Contrast {
        amount: f32, // 0.0 = 全灰, 1.0 = 正常, 2.0 = 双倍对比度
    },
    Saturation {
        amount: f32, // 0.0 = 灰度, 1.0 = 正常, 2.0 = 双倍饱和
    },
    HueRotate {
        degrees: f32, // 0-360
    },
    Grayscale {
        amount: f32, // 0.0 = 彩色, 1.0 = 完全灰度
    },
    Sepia {
        amount: f32, // 0.0 = 正常, 1.0 = 完全褐色
    },
    Invert {
        amount: f32, // 0.0 = 正常, 1.0 = 完全反转
    },
}

pub enum RenderCommand {
    /// 标准绘制指令
    Draw {
        pipeline: RenderPipeline,
        bind_group: BindGroup,
        extra_bind_groups: Vec<BindGroup>,
        vertex_buffer: Option<Buffer>,
        index_buffer: Option<Buffer>,
        /// 绘制所需的 Uniform/Instance 数据缓冲
        /// 由 Renderer 预先写入（使用 StagingBelt），Queue 只负责绑定
        instance_buffer: Option<Buffer>,
        /// 绘制索引数/顶点数
        count: u32,
    },

    /// 裁剪指令
    BeginClip {
        rect: Rect,
    },
    EndClip,

    /// 强制提交当前 RenderPass，开始新的渲染周期
    /// 用于在需要纹理操作时打断当前 pass
    Barrier,

    /// 捕获背景并应用滤镜（在 Barrier 之后执行）
    CaptureBackdrop {
        source_view: TextureView,
        final_view: TextureView,
        intermediate_view: TextureView,
        rect: Rect,
        filters: Vec<FilterKind>,
    },

    /// 离屏渲染（用于滤镜）
    BeginOffscreenPass {
        offscreen_view: TextureView,
        rect: Rect,
    },
    EndOffscreenPass {
        offscreen_view: TextureView,
        final_view: TextureView,
        intermediate_view: TextureView,
        rect: Rect,
        filters: Vec<FilterKind>,
    },
}

pub struct RenderQueue {
    pub commands: Vec<RenderCommand>,
}

impl RenderQueue {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn push(&mut self, command: RenderCommand) {
        self.commands.push(command);
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }
}
