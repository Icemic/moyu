mod bmp;
mod jpeg;
mod jxl;
mod png;
mod webp;

use crate::Rgba8Image;
use crate::error::{ImageError, Result};

pub(crate) use jxl::JxlAnimationDecoder;

pub(crate) const MAX_DECODE_BYTES: usize = 512 * 1024 * 1024;

pub(crate) fn decode(data: &[u8]) -> Result<Rgba8Image> {
    match detect_format(data) {
        Some(Format::Png) => png::decode(data),
        Some(Format::WebP) => webp::decode(data),
        Some(Format::Jpeg) => jpeg::decode(data),
        Some(Format::Jxl) => jxl::decode(data),
        Some(Format::Bmp) => bmp::decode(data),
        None => Err(ImageError::new("unsupported image format")),
    }
}

pub(crate) fn encode_webp(image: &Rgba8Image) -> Result<Vec<u8>> {
    webp::encode(image)
}

pub(crate) fn rgba8_len(width: u32, height: u32) -> Result<usize> {
    let len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(ImageError::invalid_layout)?;

    if len > MAX_DECODE_BYTES {
        return Err(ImageError::new("decoded image exceeds 512 MiB limit"));
    }

    Ok(len)
}

pub(crate) fn rgba8_stride(width: u32) -> Result<u32> {
    width.checked_mul(4).ok_or_else(ImageError::invalid_layout)
}

enum Format {
    Png,
    WebP,
    Jpeg,
    Jxl,
    Bmp,
}

fn detect_format(data: &[u8]) -> Option<Format> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(Format::Png)
    } else if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
        Some(Format::WebP)
    } else if data.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(Format::Jpeg)
    } else if jxl::is_jxl(data) {
        Some(Format::Jxl)
    } else if data.starts_with(b"BM") {
        Some(Format::Bmp)
    } else {
        None
    }
}
