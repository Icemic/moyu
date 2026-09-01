use crate::Rgba8Image;
use crate::error::{ImageError, Result};

pub(crate) fn resize(image: &Rgba8Image, width: u32, height: u32) -> Result<Rgba8Image> {
    let stride = width
        .checked_mul(4)
        .ok_or_else(ImageError::invalid_layout)?;
    let len = (stride as usize)
        .checked_mul(height as usize)
        .ok_or_else(ImageError::invalid_layout)?;
    let mut destination = Rgba8Image::from_rgba8(width, height, stride, vec![0; len])?;

    fast_image_resize::resize_u8x4_lanczos3(image, &mut destination);

    Ok(destination)
}
