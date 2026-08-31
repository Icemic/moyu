use std::io::Cursor;
use std::time::Duration;

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

pub(crate) struct WebPAnimationDecoder {
    data: Vec<u8>,
    decoder: image_webp::WebPDecoder<Cursor<Vec<u8>>>,
    frame: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) canvas: Vec<u8>,
}

impl WebPAnimationDecoder {
    pub(crate) fn new(data: Vec<u8>) -> Result<Self> {
        let decoder = Self::create_decoder(&data)?;
        let (width, height) = decoder.dimensions();
        let canvas_len = rgba8_len(width, height)?;
        let frame = vec![
            0;
            decoder.output_buffer_size().ok_or_else(|| {
                ImageError::new("WebP image dimensions exceed addressable memory")
            })?
        ];
        let canvas = vec![0; canvas_len];

        Ok(Self {
            data,
            decoder,
            width,
            height,
            frame,
            canvas,
        })
    }

    pub(crate) fn create_decoder(data: &[u8]) -> Result<image_webp::WebPDecoder<Cursor<Vec<u8>>>> {
        let decoder =
            image_webp::WebPDecoder::new(Cursor::new(data.to_vec())).map_err(|error| {
                ImageError::with_source("failed to read animated WebP header", error)
            })?;
        if !decoder.is_animated() {
            return Err(ImageError::new("WebP image is not animated"));
        }
        Ok(decoder)
    }

    pub(crate) fn advance(&mut self) -> Result<Option<Duration>> {
        let duration = match self.decoder.read_frame(&mut self.frame) {
            Ok(duration) => duration,
            Err(image_webp::DecodingError::NoMoreFrames) => return Ok(None),
            Err(error) => {
                return Err(ImageError::with_source(
                    "failed to decode WebP frame",
                    error,
                ));
            }
        };

        if self.decoder.has_alpha() {
            self.canvas.copy_from_slice(&self.frame);
        } else {
            for (source, destination) in self
                .frame
                .chunks_exact(3)
                .zip(self.canvas.chunks_exact_mut(4))
            {
                destination.copy_from_slice(&[source[0], source[1], source[2], 255]);
            }
        }
        Ok(Some(Duration::from_millis(u64::from(duration))))
    }

    pub(crate) fn restart(&mut self) -> Result<()> {
        let decoder = Self::create_decoder(&self.data)?;
        let (width, height) = decoder.dimensions();
        let canvas_len = rgba8_len(width, height)?;
        let frame = vec![
            0;
            decoder.output_buffer_size().ok_or_else(|| {
                ImageError::new("WebP image dimensions exceed addressable memory")
            })?
        ];

        self.decoder = decoder;
        self.width = width;
        self.height = height;
        self.frame = frame;
        self.canvas = vec![0; canvas_len];
        Ok(())
    }
}
