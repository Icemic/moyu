use std::collections::HashMap;
use wgpu::{
    Device, Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor,
};

const TIMEOUT_SECONDS: f64 = 5.0;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
struct TextureKey {
    width: u32,
    height: u32,
    format: TextureFormat,
    usage: TextureUsages,
}

pub struct PooledTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub last_used: f64,
}

pub struct TexturePool {
    available: HashMap<TextureKey, Vec<PooledTexture>>,
}

impl TexturePool {
    pub fn new() -> Self {
        Self {
            available: HashMap::new(),
        }
    }

    pub fn acquire(
        &mut self,
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
        timestamp: f64,
    ) -> PooledTexture {
        let key = TextureKey {
            width,
            height,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        };

        if let Some(list) = self.available.get_mut(&key) {
            if let Some(mut resource) = list.pop() {
                resource.last_used = timestamp;
                return resource;
            }
        }

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Pooled Filter Texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: key.usage,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());

        PooledTexture {
            texture,
            view,
            last_used: timestamp,
        }
    }

    pub fn return_texture(&mut self, texture: PooledTexture) {
        let size = texture.texture.size();
        let key = TextureKey {
            width: size.width,
            height: size.height,
            format: texture.texture.format(),
            usage: texture.texture.usage(),
        };

        self.available.entry(key).or_default().push(texture);
    }

    pub fn cleanup(&mut self, current_time: f64) {
        self.available.retain(|_, list| {
            let mut i = 0;
            while i < list.len() {
                if (current_time - list[i].last_used) >= TIMEOUT_SECONDS {
                    let pooled = list.swap_remove(i);
                    pooled.texture.destroy();
                } else {
                    i += 1;
                }
            }
            !list.is_empty()
        });
    }
}

impl Default for TexturePool {
    fn default() -> Self {
        Self::new()
    }
}
