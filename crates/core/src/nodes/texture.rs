use arc_swap::{ArcSwap, ArcSwapOption};
use std::sync::Arc;

#[derive(Debug)]
pub struct Texture {
    pub status: ArcSwap<TextureStatus>,
    pub texture: ArcSwapOption<wgpu::Texture>,
    pub view: ArcSwapOption<wgpu::TextureView>,
    pub sampler: ArcSwapOption<wgpu::Sampler>,
}

impl Default for Texture {
    fn default() -> Self {
        Self::new()
    }
}

impl Texture {
    pub fn new() -> Self {
        Self {
            status: ArcSwap::default(),
            texture: ArcSwapOption::default(),
            view: ArcSwapOption::default(),
            sampler: ArcSwapOption::default(),
        }
    }

    pub fn size(&self) -> (u32, u32) {
        if let Some(texture) = self.texture.load().as_ref() {
            (texture.width(), texture.height())
        } else {
            (0, 0)
        }
    }

    pub fn status(&self) -> TextureStatus {
        *self.status.load().as_ref()
    }

    pub fn set_status(&self, status: TextureStatus) {
        self.status.store(Arc::new(status));
    }

    pub fn set_texture(
        &self,
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        sampler: wgpu::Sampler,
    ) {
        self.texture.store(Some(Arc::new(texture)));
        self.view.store(Some(Arc::new(view)));
        self.sampler.store(Some(Arc::new(sampler)));
    }

    pub fn texture_unwrap(&self) -> Arc<wgpu::Texture> {
        self.texture.load().clone().unwrap()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TextureStatus {
    /// reading image file from file system
    #[default]
    Reading,
    /// uploading to graphic memory, aks creating wgpu::Texture
    Uploading,
    /// ready to read and render
    Ready,
    /// something occurs
    Error,
}
