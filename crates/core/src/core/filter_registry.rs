use crate::core::render_command::FilterKind;
use crate::core::texture_pool::{PooledTexture, TexturePool};
use crate::traits::FilterRenderer;
use std::collections::HashMap;
use wgpu::*;

pub struct FilterRegistry {
    renderers: HashMap<String, Box<dyn FilterRenderer>>,
}

impl FilterRegistry {
    pub fn new() -> Self {
        Self {
            renderers: HashMap::new(),
        }
    }

    pub fn register(&mut self, renderer: Box<dyn FilterRenderer>) {
        self.renderers.insert(renderer.name().to_string(), renderer);
    }

    pub fn get(&mut self, name: &str) -> Option<&mut Box<dyn FilterRenderer>> {
        self.renderers.get_mut(name)
    }

    /// 重置所有 renderer 的帧内状态
    pub fn reset_all_frames(&mut self) {
        for renderer in self.renderers.values_mut() {
            renderer.reset_frame();
        }
    }

    /// 执行滤镜链
    pub fn execute_filter_chain(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        input: &TextureView,
        output: &TextureView,
        filters: &[FilterKind],
        width: u32,
        height: u32,
        scale: f32,
        format: TextureFormat,
        pool: &mut TexturePool,
        timestamp: f64,
    ) {
        if filters.is_empty() {
            return;
        }

        let mut current_pooled_input: Option<PooledTexture> = None;

        for (i, filter) in filters.iter().enumerate() {
            let is_last = i == filters.len() - 1;
            let renderer_name = match filter {
                FilterKind::BlurPerfect { .. } => "blur-perfect",
                FilterKind::Blur { .. } => "blur",
                FilterKind::Brightness { .. }
                | FilterKind::Contrast { .. }
                | FilterKind::Saturation { .. }
                | FilterKind::HueRotate { .. }
                | FilterKind::Grayscale { .. }
                | FilterKind::Sepia { .. }
                | FilterKind::Invert { .. } => "color-adjust",
                FilterKind::Unknown => "unknown",
            };

            if renderer_name == "unknown" {
                log::warn!("Unknown filter kind: {:?}", filter);
                continue;
            }

            let Some(renderer) = self.get(renderer_name) else {
                log::error!("Filter renderer '{}' not found", renderer_name);
                continue;
            };

            let src_view = match &current_pooled_input {
                Some(t) => &t.view,
                None => input,
            };

            if is_last {
                renderer.execute(
                    device, queue, encoder, src_view, output, filter, width, height, scale, pool,
                    timestamp,
                );
            } else {
                let dest_pooled = pool.acquire(device, width, height, format, timestamp);
                renderer.execute(
                    device,
                    queue,
                    encoder,
                    src_view,
                    &dest_pooled.view,
                    filter,
                    width,
                    height,
                    scale,
                    pool,
                    timestamp,
                );

                if let Some(t) = current_pooled_input {
                    pool.return_texture(t);
                }
                current_pooled_input = Some(dest_pooled);
            }
        }

        if let Some(t) = current_pooled_input {
            pool.return_texture(t);
        }
    }
}
