use fast_image_resize as fr;

use crate::Rgba8Image;
use crate::error::{ImageError, Result};

pub(crate) fn resize(image: &Rgba8Image, width: u32, height: u32) -> Result<Rgba8Image> {
    let source = fr::images::Image::from_vec_u8(
        image.width(),
        image.height(),
        image.to_compact().into_owned(),
        fr::PixelType::U8x4,
    )
    .map_err(|error| ImageError::with_source("invalid RGBA8 resize input", error))?;
    let mut destination = fr::images::Image::new(width, height, fr::PixelType::U8x4);
    let options = fr::ResizeOptions::new()
        .resize_alg(fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3))
        .use_alpha(false);

    fr::Resizer::new()
        .resize(&source, &mut destination, &options)
        .map_err(|error| ImageError::with_source("failed to resize RGBA8 image", error))?;

    let stride = width
        .checked_mul(4)
        .ok_or_else(ImageError::invalid_layout)?;
    Rgba8Image::from_rgba8(width, height, stride, destination.into_vec())
}
