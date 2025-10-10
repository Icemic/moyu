use image::GenericImageView;
use log::debug;
use moyu_pal::dir::assets_dir;
use moyu_pal::{fs, task};
use std::sync::Arc;
use wgpu::{Device, Queue};

use crate::types::{Texture, TextureStatus};
use crate::utils::premultiply_alpha;

pub(crate) fn load_texture(device: &Device, queue: &Queue, src: &str) -> Arc<Texture> {
    let src_full = assets_dir().join(src).unwrap();

    debug!("loading texture from {}", src);

    let texture = Arc::new(Texture::new());

    {
        let device = device.clone();
        let queue = queue.clone();
        let texture = texture.clone();
        let src = src.to_owned();
        let task_fn = async move {
            let bytes = match fs::read(&src_full).await {
                Ok(v) => v,
                Err(err) => {
                    log::error!("Failed to read '{}': {}", src, err);
                    return Err(anyhow::format_err!("Failed to read '{}': {}", src, err));
                }
            };

            load_image_to_texture(&texture, &device, &queue, &bytes, Some(&src))?;

            debug!("texture '{}' loaded", src);

            Ok(())
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

    let texture_gpu = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        view_formats: &[],
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
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

    Ok(())
}
