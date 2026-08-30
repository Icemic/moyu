use std::io::Cursor;
use std::time::Duration;

use crate::codecs::rgba8_len;
use crate::error::{ImageError, Result};
use crate::ops;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationFormat {
    Apng,
    WebP,
}

pub struct AnimationDecoder {
    inner: AnimationDecoderInner,
}

enum AnimationDecoderInner {
    Apng(ApngDecoder),
    WebP(WebPAnimationDecoder),
}

impl AnimationDecoder {
    pub fn new(data: Vec<u8>, format: AnimationFormat) -> Result<Self> {
        let inner = match format {
            AnimationFormat::Apng => AnimationDecoderInner::Apng(ApngDecoder::new(data)?),
            AnimationFormat::WebP => AnimationDecoderInner::WebP(WebPAnimationDecoder::new(data)?),
        };
        Ok(Self { inner })
    }

    pub fn width(&self) -> u32 {
        match &self.inner {
            AnimationDecoderInner::Apng(decoder) => decoder.width,
            AnimationDecoderInner::WebP(decoder) => decoder.width,
        }
    }

    pub fn height(&self) -> u32 {
        match &self.inner {
            AnimationDecoderInner::Apng(decoder) => decoder.height,
            AnimationDecoderInner::WebP(decoder) => decoder.height,
        }
    }

    pub fn advance(&mut self) -> Result<Option<Duration>> {
        match &mut self.inner {
            AnimationDecoderInner::Apng(decoder) => decoder.advance(),
            AnimationDecoderInner::WebP(decoder) => decoder.advance(),
        }
    }

    pub fn write_premultiplied_frame(&self, output: &mut Vec<u8>) -> Result<()> {
        match &self.inner {
            AnimationDecoderInner::Apng(decoder) => {
                ops::copy_premultiplied_rgba8(&decoder.canvas, output)
            }
            AnimationDecoderInner::WebP(decoder) => {
                ops::copy_premultiplied_rgba8(&decoder.canvas, output)
            }
        }
        Ok(())
    }

    pub fn restart(&mut self) -> Result<()> {
        match &mut self.inner {
            AnimationDecoderInner::Apng(decoder) => decoder.restart(),
            AnimationDecoderInner::WebP(decoder) => decoder.restart(),
        }
    }
}

struct ApngDecoder {
    data: Vec<u8>,
    reader: png::Reader<Cursor<Vec<u8>>>,
    width: u32,
    height: u32,
    canvas: Vec<u8>,
    frame: Vec<u8>,
    pending: Option<ApngFrameState>,
    remaining_frames: u32,
}

struct ApngFrameState {
    control: png::FrameControl,
    previous_canvas: Option<Vec<u8>>,
}

impl ApngDecoder {
    fn new(data: Vec<u8>) -> Result<Self> {
        let (reader, width, height, canvas, remaining_frames) = Self::create_reader(&data)?;
        let frame = vec![
            0;
            reader.output_buffer_size().ok_or_else(|| {
                ImageError::new("APNG image dimensions exceed addressable memory")
            })?
        ];

        Ok(Self {
            data,
            reader,
            width,
            height,
            canvas,
            frame,
            pending: None,
            remaining_frames,
        })
    }

    fn create_reader(
        data: &[u8],
    ) -> Result<(png::Reader<Cursor<Vec<u8>>>, u32, u32, Vec<u8>, u32)> {
        let mut decoder = png::Decoder::new(Cursor::new(data.to_vec()));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let reader = decoder
            .read_info()
            .map_err(|error| ImageError::with_source("failed to read APNG header", error))?;
        let animation = reader
            .info()
            .animation_control
            .ok_or_else(|| ImageError::new("PNG image is not animated"))?;
        let width = reader.info().width;
        let height = reader.info().height;
        let canvas = vec![0; rgba8_len(width, height)?];
        Ok((reader, width, height, canvas, animation.num_frames))
    }

    fn advance(&mut self) -> Result<Option<Duration>> {
        if self.remaining_frames == 0 {
            return Ok(None);
        }
        if let Some(frame) = self.pending.take() {
            self.dispose(frame);
        }

        let output = match self.reader.next_frame(&mut self.frame) {
            Ok(output) => output,
            Err(error) => {
                return Err(ImageError::with_source(
                    "failed to decode APNG frame",
                    error,
                ));
            }
        };
        let control = self
            .reader
            .info()
            .frame_control
            .ok_or_else(|| ImageError::new("APNG frame is missing control data"))?;
        let frame_len = (output.width as usize)
            .checked_mul(output.height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(ImageError::invalid_layout)?;
        let frame = match output.color_type {
            png::ColorType::Rgba => self.frame[..frame_len].to_vec(),
            png::ColorType::Rgb => rgb_to_rgba(&self.frame, output.width, output.height),
            png::ColorType::Grayscale => gray_to_rgba(&self.frame, output.width, output.height),
            png::ColorType::GrayscaleAlpha => {
                gray_alpha_to_rgba(&self.frame, output.width, output.height)
            }
            png::ColorType::Indexed => {
                return Err(ImageError::new("APNG palette was not expanded"));
            }
        };
        let previous_canvas =
            (control.dispose_op == png::DisposeOp::Previous).then(|| self.canvas.clone());
        self.blend(&control, &frame)?;
        self.pending = Some(ApngFrameState {
            control,
            previous_canvas,
        });
        self.remaining_frames -= 1;

        Ok(Some(frame_duration(control.delay_num, control.delay_den)))
    }

    fn blend(&mut self, control: &png::FrameControl, frame: &[u8]) -> Result<()> {
        if control.x_offset + control.width > self.width
            || control.y_offset + control.height > self.height
        {
            return Err(ImageError::new("APNG frame exceeds animation canvas"));
        }

        for y in 0..control.height as usize {
            let destination_row = ((control.y_offset as usize + y) * self.width as usize
                + control.x_offset as usize)
                * 4;
            let source_row = y * control.width as usize * 4;
            let destination =
                &mut self.canvas[destination_row..destination_row + control.width as usize * 4];
            let source = &frame[source_row..source_row + control.width as usize * 4];

            match control.blend_op {
                png::BlendOp::Source => destination.copy_from_slice(source),
                png::BlendOp::Over => {
                    for (destination, source) in
                        destination.chunks_exact_mut(4).zip(source.chunks_exact(4))
                    {
                        blend_over(destination, source);
                    }
                }
            }
        }
        Ok(())
    }

    fn dispose(&mut self, frame: ApngFrameState) {
        match frame.control.dispose_op {
            png::DisposeOp::None => {}
            png::DisposeOp::Background => {
                for y in 0..frame.control.height as usize {
                    let start = ((frame.control.y_offset as usize + y) * self.width as usize
                        + frame.control.x_offset as usize)
                        * 4;
                    self.canvas[start..start + frame.control.width as usize * 4].fill(0);
                }
            }
            png::DisposeOp::Previous => {
                self.canvas = frame
                    .previous_canvas
                    .expect("APNG previous disposal has saved canvas");
            }
        }
    }

    fn restart(&mut self) -> Result<()> {
        let (reader, width, height, canvas, remaining_frames) = Self::create_reader(&self.data)?;
        let frame = vec![
            0;
            reader.output_buffer_size().ok_or_else(|| {
                ImageError::new("APNG image dimensions exceed addressable memory")
            })?
        ];
        self.reader = reader;
        self.width = width;
        self.height = height;
        self.canvas = canvas;
        self.frame = frame;
        self.pending = None;
        self.remaining_frames = remaining_frames;
        Ok(())
    }
}

struct WebPAnimationDecoder {
    data: Vec<u8>,
    decoder: image_webp::WebPDecoder<Cursor<Vec<u8>>>,
    width: u32,
    height: u32,
    frame: Vec<u8>,
    canvas: Vec<u8>,
}

impl WebPAnimationDecoder {
    fn new(data: Vec<u8>) -> Result<Self> {
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

    fn create_decoder(data: &[u8]) -> Result<image_webp::WebPDecoder<Cursor<Vec<u8>>>> {
        let decoder =
            image_webp::WebPDecoder::new(Cursor::new(data.to_vec())).map_err(|error| {
                ImageError::with_source("failed to read animated WebP header", error)
            })?;
        if !decoder.is_animated() {
            return Err(ImageError::new("WebP image is not animated"));
        }
        Ok(decoder)
    }

    fn advance(&mut self) -> Result<Option<Duration>> {
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

    fn restart(&mut self) -> Result<()> {
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

fn frame_duration(numerator: u16, denominator: u16) -> Duration {
    let denominator = u64::from(if denominator == 0 { 100 } else { denominator });
    Duration::from_nanos(u64::from(numerator) * 1_000_000_000 / denominator)
}

fn rgb_to_rgba(source: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for pixel in source
        .chunks_exact(3)
        .take(width as usize * height as usize)
    {
        rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
    }
    rgba
}

fn gray_to_rgba(source: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for &value in source.iter().take(width as usize * height as usize) {
        rgba.extend_from_slice(&[value, value, value, 255]);
    }
    rgba
}

fn gray_alpha_to_rgba(source: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for pixel in source
        .chunks_exact(2)
        .take(width as usize * height as usize)
    {
        rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
    }
    rgba
}

fn blend_over(destination: &mut [u8], source: &[u8]) {
    let source_alpha = source[3] as f32 / 255.0;
    let destination_alpha = destination[3] as f32 / 255.0;
    let alpha = source_alpha + destination_alpha * (1.0 - source_alpha);

    if alpha == 0.0 {
        destination.fill(0);
        return;
    }

    destination[0] = ((source[0] as f32 * source_alpha
        + destination[0] as f32 * destination_alpha * (1.0 - source_alpha))
        / alpha) as u8;
    destination[1] = ((source[1] as f32 * source_alpha
        + destination[1] as f32 * destination_alpha * (1.0 - source_alpha))
        / alpha) as u8;
    destination[2] = ((source[2] as f32 * source_alpha
        + destination[2] as f32 * destination_alpha * (1.0 - source_alpha))
        / alpha) as u8;
    destination[3] = (alpha * 255.0) as u8;
}
