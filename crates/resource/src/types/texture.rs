use arc_swap::{ArcSwap, ArcSwapOption};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug)]
pub struct Texture {
    pub status: ArcSwap<TextureStatus>,
    pub texture: ArcSwapOption<wgpu::Texture>,
    pub view: ArcSwapOption<wgpu::TextureView>,
    /// Pixel ratio of the loaded file, `1.0` for the base asset.
    ///
    /// Renderers divide texture pixel sizes by this value so that the on-screen
    /// size does not depend on which multi-resolution variant was loaded.
    /// Stored as IEEE-754 bits so that reads and writes stay lock-free.
    pixel_ratio: AtomicU32,
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
            pixel_ratio: AtomicU32::new(1.0f32.to_bits()),
        }
    }

    /// Pixel ratio of the loaded file, `1.0` for the base asset.
    pub fn pixel_ratio(&self) -> f32 {
        f32::from_bits(self.pixel_ratio.load(Ordering::Relaxed))
    }

    pub fn set_pixel_ratio(&self, ratio: f32) {
        self.pixel_ratio.store(ratio.to_bits(), Ordering::Relaxed);
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

    pub fn set_texture(&self, texture: wgpu::Texture, view: wgpu::TextureView) {
        self.texture.store(Some(Arc::new(texture)));
        self.view.store(Some(Arc::new(view)));
    }

    pub fn texture_unwrap(&self) -> Arc<wgpu::Texture> {
        self.texture.load().clone().unwrap()
    }
}

// impl Drop for Texture {
//     fn drop(&mut self) {
//         if let Some(texture) = self.texture.load().as_ref() {
//             texture.destroy();
//         }
//     }
// }

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
