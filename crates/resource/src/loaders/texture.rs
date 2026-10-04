use log::debug;
use moyu_image::{Rgba8Image, decode};
use moyu_pal::task;
use std::sync::Arc;
use wgpu::{Device, Queue};

use crate::mipmap::MipmapGenerator;
use crate::types::{Texture, TextureStatus};
use crate::variant;

/// Load a texture, using the multi-resolution variant that matches the current
/// display ratio.
///
/// `src` is the logical asset path the project references, relative to the assets
/// directory.
pub(crate) fn load_texture(
    device: &Device,
    queue: &Queue,
    src: &str,
    mipmap_generator: Option<Arc<MipmapGenerator>>,
) -> Arc<Texture> {
    let texture = Arc::new(Texture::new());

    {
        let device = device.clone();
        let queue = queue.clone();
        let texture = texture.clone();
        let src = src.to_owned();
        let task_fn = async move {
            let Some((url, scale, image)) =
                variant::load_variant(&src, |bytes| decode(&bytes).ok()).await
            else {
                log::error!("failed to load texture '{}'", src);
                texture.set_status(TextureStatus::Error);
                return;
            };

            texture.set_pixel_ratio(scale);

            if let Err(err) = upload_image(
                &texture,
                &device,
                &queue,
                image,
                Some(url.as_str()),
                mipmap_generator.as_deref(),
            ) {
                log::error!("failed to upload texture '{}' ({}): {}", src, url, err);
                texture.set_status(TextureStatus::Error);
            } else {
                debug!("texture '{}' loaded from '{}' (scale {})", src, url, scale);
            }
        };

        task::spawn(task_fn);
    }

    texture
}

/// Decode image bytes and upload them into the texture.
pub(crate) fn load_image_to_texture(
    texture: &Arc<Texture>,
    device: &Device,
    queue: &Queue,
    bytes: &[u8],
    label: Option<&str>,
    mipmap_generator: Option<&MipmapGenerator>,
) -> anyhow::Result<()> {
    let image = decode(bytes)?;

    upload_image(texture, device, queue, image, label, mipmap_generator)
}

/// Upload an already decoded image into the texture.
pub(crate) fn upload_image(
    texture: &Arc<Texture>,
    device: &Device,
    queue: &Queue,
    mut image: Rgba8Image,
    label: Option<&str>,
    mipmap_generator: Option<&MipmapGenerator>,
) -> anyhow::Result<()> {
    let dimensions = (image.width(), image.height());

    image.premultiply_alpha_in_place();
    let alpha_mask = image.extract_alpha_mask();
    let rgba = image.into_data();

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

    // Publish the mask before the texture turns ready, so hit testing can rely on
    // `Ready` implying an available mask.
    texture.set_alpha_mask(alpha_mask);
    texture.set_texture(texture_gpu, view);
    texture.set_status(TextureStatus::Ready);

    Ok(())
}
