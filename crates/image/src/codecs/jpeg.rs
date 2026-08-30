use crate::Rgba8Image;
use crate::codecs::{rgba8_len, rgba8_stride};
use crate::error::{ImageError, Result};

const MAX_JPEG_DIMENSION: usize = 16_384;
const MAX_JPEG_SCANS: usize = 100;

pub(crate) fn decode(data: &[u8]) -> Result<Rgba8Image> {
    let options = zune_core::options::DecoderOptions::default()
        .set_max_width(MAX_JPEG_DIMENSION)
        .set_max_height(MAX_JPEG_DIMENSION)
        .jpeg_set_max_scans(MAX_JPEG_SCANS)
        .jpeg_set_out_colorspace(zune_core::colorspace::ColorSpace::RGB);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(
        zune_core::bytestream::ZCursor::new(data),
        options,
    );
    decoder
        .decode_headers()
        .map_err(|error| ImageError::with_source("failed to read JPEG header", error))?;
    let info = decoder
        .info()
        .ok_or_else(|| ImageError::new("JPEG header does not contain image dimensions"))?;
    let width = u32::from(info.width);
    let height = u32::from(info.height);
    let expected_len = rgba8_len(width, height)?;
    let rgb = decoder
        .decode()
        .map_err(|error| ImageError::with_source("failed to decode JPEG image", error))?;
    let mut rgba = Vec::with_capacity(expected_len);

    for pixel in rgb.chunks_exact(3) {
        rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
    }

    Rgba8Image::from_rgba8(width, height, rgba8_stride(width)?, rgba)
}
