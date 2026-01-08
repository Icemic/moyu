use crate::core::render_command::FilterKind;
use crate::traits::FilterRenderer;
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::*;

pub struct FilterRegistry {
    renderers: HashMap<String, Arc<dyn FilterRenderer>>,
}

impl FilterRegistry {
    pub fn new() -> Self {
        Self {
            renderers: HashMap::new(),
        }
    }

    pub fn register(&mut self, renderer: Arc<dyn FilterRenderer>) {
        self.renderers.insert(renderer.name().to_string(), renderer);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn FilterRenderer>> {
        self.renderers.get(name)
    }

    /// 执行滤镜链
    pub fn execute_filter_chain(
        &self,
        device: &Device,
        encoder: &mut CommandEncoder,
        input: &TextureView,
        output: &TextureView,
        filters: &[FilterKind],
        width: u32,
        height: u32,
        // TODO: 纹理池支持，目前简单实现
        intermediate_textures: &Vec<TextureView>,
    ) {
        if filters.is_empty() {
            // 无滤镜，理论上不应该到这里，或者应该执行 blit
            return;
        }

        let mut current_input = input;

        for (i, filter) in filters.iter().enumerate() {
            let is_last = i == filters.len() - 1;

            // 获取对应的 renderer
            let renderer_name = match filter {
                FilterKind::BlurPerfect { .. } => "blur-perfect",
                FilterKind::Blur { .. } => "blur",
                FilterKind::Brightness { .. }
                | FilterKind::Contrast { .. }
                | FilterKind::Saturation { .. } => "color_adjust",
                FilterKind::HueRotate { .. } => "hue_rotate",
                FilterKind::Grayscale { .. } => "grayscale",
                FilterKind::Sepia { .. } => "sepia",
                FilterKind::Invert { .. } => "invert",
            };

            let renderer = self
                .get(renderer_name)
                .expect(&format!("Filter renderer '{}' not found", renderer_name));

            if is_last {
                renderer.execute(
                    device,
                    encoder,
                    current_input,
                    output,
                    filter,
                    width,
                    height,
                );
            } else {
                // 需要一个中间纹理
                // 这里暂时假设 intermediate_textures 已经准备好了足够的纹理
                // 实际应该从纹理池获取
                let temp_view = &intermediate_textures[i % intermediate_textures.len()];
                renderer.execute(
                    device,
                    encoder,
                    current_input,
                    temp_view,
                    filter,
                    width,
                    height,
                );
                current_input = temp_view;
            }
        }
    }
}
