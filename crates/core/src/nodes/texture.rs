use hai_pal::sync::RwLock;
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct Texture {
    pub status: TextureStatus,
    pub texture: Option<wgpu::Texture>,
    pub view: Option<wgpu::TextureView>,
    pub sampler: Option<wgpu::Sampler>,
}

impl Texture {
    pub fn new() -> Self {
        Self {
            status: Default::default(),
            texture: None,
            view: None,
            sampler: None,
        }
    }

    pub fn width(&self) -> u32 {
        if let Some(ref texture) = self.texture {
            texture.width()
        } else {
            0
        }
    }

    pub fn height(&self) -> u32 {
        if let Some(ref texture) = self.texture {
            texture.height()
        } else {
            0
        }
    }

    pub fn status(&self) -> &TextureStatus {
        &self.status
    }

    pub fn set_status(&mut self, status: TextureStatus) {
        self.status = status;
    }

    pub fn set_texture(
        &mut self,
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        sampler: wgpu::Sampler,
    ) {
        self.texture = Some(texture);
        self.view = Some(view);
        self.sampler = Some(sampler);
    }

    pub fn texture_unwrap(&self) -> &wgpu::Texture {
        self.texture.as_ref().unwrap()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Default)]
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

static EMPTY_TEXTURE: OnceCell<Arc<RwLock<Texture>>> = OnceCell::new();

pub fn get_empty_texture() -> &'static Arc<RwLock<Texture>> {
    EMPTY_TEXTURE.get_or_init(|| Arc::new(RwLock::new(Texture::new())))
}
