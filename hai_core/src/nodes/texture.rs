use hai_pal::sync::RwLock;
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct Texture {
    pub status: TextureStatus,
    pub texture: Option<wgpu::Texture>,
    pub view: Option<wgpu::TextureView>,
    pub sampler: Option<wgpu::Sampler>,
    pub width: u32,
    pub height: u32,
}

impl Texture {
    pub fn new() -> Self {
        Self {
            status: Default::default(),
            texture: None,
            view: None,
            sampler: None,
            width: 0,
            height: 0,
        }
    }

    pub fn status(&self) -> TextureStatus {
        self.status.clone()
    }

    pub fn set_status(&mut self, status: TextureStatus) {
        self.status = status;
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
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
