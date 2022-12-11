use anyhow::Result;
use futures::{stream::FuturesUnordered, task::AtomicWaker, Future, FutureExt, StreamExt};
use hai_pal::env::entry_dir;
use hai_pal::fs;
use image::GenericImageView;
use log::{debug, error};
use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex, RwLock, Weak},
    task::{Context, Poll},
};
use wgpu::{Device, Queue};

use crate::nodes::{Texture, TextureStatus};

pub struct ResourceManager {
    device: Arc<Mutex<Device>>,
    queue: Arc<Mutex<Queue>>,
    texture_map: HashMap<String, Weak<RwLock<Texture>>>,
    #[cfg(not(target_arch = "wasm32"))]
    tasks: FuturesUnordered<Pin<Box<dyn Future<Output = Result<()>> + Send>>>,
    #[cfg(target_arch = "wasm32")]
    tasks: FuturesUnordered<Pin<Box<dyn Future<Output = Result<()>>>>>,
    waker: AtomicWaker,
}

impl ResourceManager {
    pub fn new(device: Arc<Mutex<Device>>, queue: Arc<Mutex<Queue>>) -> Self {
        Self {
            device,
            queue,
            texture_map: Default::default(),
            tasks: Default::default(),
            waker: Default::default(),
        }
    }

    /// get a texture
    /// if there's already a texture with the same asset path, return it,
    /// or it will add a new task to load.
    pub fn get_texture(&mut self, asset_relative_path: String) -> Arc<RwLock<Texture>> {
        if let Some(texture) = self.texture_map.get(&asset_relative_path) {
            if let Some(texture) = texture.upgrade() {
                return texture;
            }
        }
        self.add_task(asset_relative_path)
    }

    /// add a task to load a new texture.
    /// it does not check whether a same asset has been loaded.
    pub fn add_task(&mut self, asset_relative_path: String) -> Arc<RwLock<Texture>> {
        let asset_full_path = entry_dir()
            .join("assets/")
            .unwrap()
            .join(&asset_relative_path)
            .unwrap();
        debug!("texture will load from {}", asset_relative_path);

        let texture = Arc::new(RwLock::new(Texture::new()));
        self.texture_map
            .insert(asset_relative_path.to_string(), Arc::downgrade(&texture));
        let _texture = texture.clone();

        let device = self.device.clone();
        let queue = self.queue.clone();
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

            // TODO: map various color type to wgpu::TextureFormat
            let rgba = img
                .as_rgba8()
                .expect("failed to read image data, this image may not 32bit rgba8 format.");

            let dimensions = img.dimensions();

            {
                let mut texture = texture.write().unwrap();
                texture.set_size(dimensions.0, dimensions.1);
                texture.set_status(TextureStatus::Uploading);
            }

            let size = wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            };

            let device = device.lock().unwrap();
            let queue = queue.lock().unwrap();

            let texture_gpu = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(asset_relative_path.as_str()),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            });

            queue.write_texture(
                wgpu::ImageCopyTexture {
                    aspect: wgpu::TextureAspect::All,
                    texture: &texture_gpu,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                rgba,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
                    rows_per_image: std::num::NonZeroU32::new(dimensions.1),
                },
                size,
            );

            let view = texture_gpu.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            {
                let mut texture = texture.write().unwrap();
                texture.set_texture(texture_gpu, view, sampler);
                texture.set_status(TextureStatus::Ready);
            }

            debug!("texture '{}' loaded", asset_relative_path);

            Ok(())
        };

        #[cfg(not(target_arch = "wasm32"))]
        let task_fn = task_fn.boxed();
        #[cfg(target_arch = "wasm32")]
        let task_fn = task_fn.boxed_local();

        self.tasks.push(task_fn);

        self.waker.wake();

        _texture
    }

    pub fn poll(&mut self, cx: &mut Context) -> Poll<()> {
        self.waker.register(cx.waker());
        match self.tasks.poll_next_unpin(cx) {
            Poll::Ready(Some(Err(err))) => {
                error!("{}", err.to_string());
                Poll::Ready(())
            }
            Poll::Ready(_) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}
