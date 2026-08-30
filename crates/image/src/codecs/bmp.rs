use crate::Rgba8Image;
use crate::codecs::{rgba8_len, rgba8_stride};
use crate::error::{ImageError, Result};

pub(crate) fn decode(data: &[u8]) -> Result<Rgba8Image> {
    let mut decoder = zune_bmp::BmpDecoder::new(zune_core::bytestream::ZCursor::new(data));
    decoder
        .decode_headers()
        .map_err(|error| ImageError::new(format!("failed to read BMP header: {error:?}")))?;
    let (width, height) = decoder
        .dimensions()
        .ok_or_else(|| ImageError::new("BMP header does not contain image dimensions"))?;
    let width = u32::try_from(width).map_err(|_| ImageError::invalid_layout())?;
    let height = u32::try_from(height).map_err(|_| ImageError::invalid_layout())?;
    let expected_len = rgba8_len(width, height)?;
    let colorspace = decoder
        .colorspace()
        .ok_or_else(|| ImageError::new("BMP header does not contain pixel format"))?;
    let source = decoder
        .decode()
        .map_err(|error| ImageError::new(format!("failed to decode BMP image: {error:?}")))?;
    let mut rgba = Vec::with_capacity(expected_len);

    match colorspace {
        zune_core::colorspace::ColorSpace::RGB => {
            for pixel in source.chunks_exact(3) {
                rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
        }
        zune_core::colorspace::ColorSpace::RGBA => rgba.extend_from_slice(&source),
        zune_core::colorspace::ColorSpace::Luma => {
            for &value in &source {
                rgba.extend_from_slice(&[value, value, value, 255]);
            }
        }
        _ => return Err(ImageError::new("unsupported BMP output format")),
    }

    Rgba8Image::from_rgba8(width, height, rgba8_stride(width)?, rgba)
}
