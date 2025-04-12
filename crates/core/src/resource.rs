use moyu_pal::{config::entry_dir, sync::RwLock};
use moyu_pal::{fs, task};
use image::GenericImageView;
use log::debug;
use std::{collections::HashMap, sync::Arc};
use wgpu::{Device, Queue};

use crate::nodes::{Texture, TextureStatus};
use crate::utils::premultiply_alpha::premultiply_alpha;

pub type RelativePath = String;
pub type RendererName = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextureId {
    // asset relative path
    Path(RelativePath),
    // custom identical string
    Custom(String),
}

#[derive(Debug)]
pub struct ResourceManager {
    device: Arc<Device>,
    queue: Arc<Queue>,
    texture_map: Arc<RwLock<HashMap<Arc<TextureId>, Arc<Texture>>>>,
}

impl ResourceManager {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            device,
            queue,
            texture_map: Default::default(),
        }
    }

    pub fn try_get_texture(&self, texture_id: &Arc<TextureId>) -> Option<Arc<Texture>> {
        if let Some(texture) = self.texture_map.read().get(texture_id) {
            return Some(texture.clone());
        }

        None
    }

    /// get a texture
    /// if there's already a texture with the same texture id, return it, or:
    ///   1. for `TextureId::Path`, it will add a new task to load a new texture
    ///   2. for `TextureId::Custom`, it will create a empty texture then return
    pub fn get_texture(&self, texture_id: &Arc<TextureId>) -> Arc<Texture> {
        if let Some(texture) = self.texture_map.read().get(texture_id) {
            return texture.clone();
        }

        match &**texture_id {
            TextureId::Path(_) => self.add_load_task(texture_id.clone()),
            TextureId::Custom(_) => {
                let texture = Arc::new(Texture::new());
                self.texture_map
                    .write()
                    .insert(texture_id.clone(), texture.clone());
                texture
            }
        }
    }

    /// add a task to load a new texture.
    /// it does not check whether a same asset has been loaded.
    fn add_load_task(&self, texture_id: Arc<TextureId>) -> Arc<Texture> {
        if let TextureId::Path(asset_relative_path) = &*texture_id {
            let asset_full_path = entry_dir()
                .join("assets/")
                .unwrap()
                .join(asset_relative_path)
                .unwrap();
            debug!("texture will load from {}", asset_relative_path);

            let texture = Arc::new(Texture::new());
            self.texture_map
                .write()
                .insert(texture_id.clone(), texture.clone());
            let _texture = texture.clone();

            let device = self.device.clone();
            let queue = self.queue.clone();
            let asset_relative_path = asset_relative_path.to_owned();
            let task_fn = async move {
                let bytes = match fs::read(&asset_full_path).await {
                    Ok(v) => v,
                    Err(err) => {
                        return Err(anyhow::format_err!(
                            "failed to read '{}': {}",
                            asset_relative_path,
                            err.to_string()
                        ));
                    }
                };

                let img = image::load_from_memory(&bytes)?;

                let dimensions = img.dimensions();

                // TODO: map various color type to wgpu::TextureFormat
                let mut rgba = img.into_rgba8();

                // perform premultiply alpha
                premultiply_alpha(&mut rgba);

                texture.set_status(TextureStatus::Uploading);

                let size = wgpu::Extent3d {
                    width: dimensions.0,
                    height: dimensions.1,
                    depth_or_array_layers: 1,
                };

                let texture_gpu = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(asset_relative_path.as_str()),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_formats: &[],
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                });

                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        aspect: wgpu::TextureAspect::All,
                        texture: &texture_gpu,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                    },
                    &rgba,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * dimensions.0),
                        rows_per_image: Some(dimensions.1),
                    },
                    size,
                );

                let view = texture_gpu.create_view(&wgpu::TextureViewDescriptor::default());
                let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::FilterMode::Linear,
                    ..Default::default()
                });

                texture.set_texture(texture_gpu, view, sampler);
                texture.set_status(TextureStatus::Ready);

                debug!("texture '{}' loaded", asset_relative_path);

                Ok(())
            };

            task::spawn(task_fn);

            _texture
        } else {
            unreachable!();
        }
    }
}
