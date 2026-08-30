use std::io::Cursor;

use crate::Rgba8Image;
use crate::codecs::{rgba8_len, rgba8_stride};
use crate::error::{ImageError, Result};

pub(crate) fn decode(data: &[u8]) -> Result<Rgba8Image> {
    let mut decoder = png::Decoder::new(Cursor::new(data));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|error| ImageError::with_source("failed to read PNG header", error))?;
    let (width, height) = (reader.info().width, reader.info().height);
    let expected_len = rgba8_len(width, height)?;
    let mut source = vec![
        0;
        reader.output_buffer_size().ok_or_else(|| {
            ImageError::new("PNG image dimensions exceed addressable memory")
        })?
    ];
    let output = reader
        .next_frame(&mut source)
        .map_err(|error| ImageError::with_source("failed to decode PNG image", error))?;
    let source = &source[..output.buffer_size()];
    let mut rgba = Vec::with_capacity(expected_len);

    match output.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(source),
        png::ColorType::Rgb => {
            for pixel in source.chunks_exact(3) {
                rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
        }
        png::ColorType::Grayscale => {
            for &value in source {
                rgba.extend_from_slice(&[value, value, value, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for pixel in source.chunks_exact(2) {
                rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
            }
        }
        png::ColorType::Indexed => {
            return Err(ImageError::new("PNG palette was not expanded"));
        }
    }

    Rgba8Image::from_rgba8(width, height, rgba8_stride(width)?, rgba)
}
