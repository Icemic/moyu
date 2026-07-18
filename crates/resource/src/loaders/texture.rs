use image::GenericImageView;
use log::debug;
use moyu_pal::url::Url;
use moyu_pal::{fs, task};
use std::sync::Arc;
use wgpu::{Device, Queue};

use crate::mipmap::MipmapGenerator;
use crate::types::{Texture, TextureStatus};
use crate::utils::premultiply_alpha;

pub(crate) fn load_texture(
    device: &Device,
    queue: &Queue,
    url: &Url,
    mipmap_generator: Option<Arc<MipmapGenerator>>,
) -> Arc<Texture> {
    debug!("loading texture from {}", url);

    let texture = Arc::new(Texture::new());

    {
        let device = device.clone();
        let queue = queue.clone();
        let texture = texture.clone();
        let url = url.to_owned();
        let task_fn = async move {
            let bytes = match fs::read(&url).await {
                Ok(v) => v,
                Err(err) => {
                    log::error!("Failed to read '{}': {}", url, err);
                    return;
                }
            };

            if let Err(err) = load_image_to_texture(
                &texture,
                &device,
                &queue,
                &bytes,
                Some(url.as_str()),
                mipmap_generator.as_deref(),
            ) {
                log::error!("Failed to load image '{}': {}", url, err);
            } else {
                debug!("texture '{}' loaded", url);
            }
        };

        task::spawn(task_fn);
    }

    texture
}

pub(crate) fn load_image_to_texture(
    texture: &Arc<Texture>,
    device: &Device,
    queue: &Queue,
    bytes: &[u8],
    label: Option<&str>,
    mipmap_generator: Option<&MipmapGenerator>,
) -> anyhow::Result<()> {
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
    let mip_level_count = mipmap_generator
        .map(|_| dimensions.0.max(dimensions.1).ilog2() + 1)
        .unwrap_or(1);

    let texture_gpu = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size,
        mip_level_count,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        view_formats: &[],
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | if mipmap_generator.is_some() {
                wgpu::TextureUsages::RENDER_ATTACHMENT
            } else {
                wgpu::TextureUsages::empty()
            },
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            texture: &texture_gpu,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * dimensions.0),
            rows_per_image: Some(dimensions.1),
        },
        size,
    );

    if let Some(mipmap_generator) = mipmap_generator {
        mipmap_generator.generate(device, queue, &texture_gpu, mip_level_count);
    }

    let view = texture_gpu.create_view(&wgpu::TextureViewDescriptor::default());

    texture.set_texture(texture_gpu, view);
    texture.set_status(TextureStatus::Ready);

    Ok(())
}
