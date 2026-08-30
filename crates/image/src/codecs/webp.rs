use std::io::Cursor;

use crate::Rgba8Image;
use crate::codecs::{rgba8_len, rgba8_stride};
use crate::error::{ImageError, Result};

pub(crate) fn decode(data: &[u8]) -> Result<Rgba8Image> {
    let mut decoder = image_webp::WebPDecoder::new(Cursor::new(data))
        .map_err(|error| ImageError::with_source("failed to read WebP header", error))?;
    let (width, height) = decoder.dimensions();
    let expected_len = rgba8_len(width, height)?;
    let mut source = vec![
        0;
        decoder.output_buffer_size().ok_or_else(|| {
            ImageError::new("WebP image dimensions exceed addressable memory")
        })?
    ];
    decoder
        .read_image(&mut source)
        .map_err(|error| ImageError::with_source("failed to decode WebP image", error))?;

    if decoder.has_alpha() {
        return Rgba8Image::from_rgba8(width, height, rgba8_stride(width)?, source);
    }

    let mut rgba = Vec::with_capacity(expected_len);
    for pixel in source.chunks_exact(3) {
        rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
    }
    Rgba8Image::from_rgba8(width, height, rgba8_stride(width)?, rgba)
}

pub(crate) fn encode(image: &Rgba8Image) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    image_webp::WebPEncoder::new(&mut output)
        .encode(
            image.to_compact().as_ref(),
            image.width(),
            image.height(),
            image_webp::ColorType::Rgba8,
        )
        .map_err(|error| ImageError::with_source("failed to encode WebP image", error))?;
    Ok(output)
}
