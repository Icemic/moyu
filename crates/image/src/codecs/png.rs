use std::io::Cursor;
use std::time::Duration;

use crate::Rgba8Image;
use crate::codecs::{rgba8_len, rgba8_stride};
use crate::error::{ImageError, Result};
use crate::utils::*;

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

pub(crate) struct ApngDecoder {
    data: Vec<u8>,
    reader: png::Reader<Cursor<Vec<u8>>>,
    frame: Vec<u8>,
    pending: Option<ApngFrameState>,
    remaining_frames: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) canvas: Vec<u8>,
}

pub(crate) struct ApngFrameState {
    control: png::FrameControl,
    previous_canvas: Option<Vec<u8>>,
}

impl ApngDecoder {
    pub(crate) fn new(data: Vec<u8>) -> Result<Self> {
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

    pub(crate) fn create_reader(
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

    pub(crate) fn advance(&mut self) -> Result<Option<Duration>> {
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

    pub(crate) fn blend(&mut self, control: &png::FrameControl, frame: &[u8]) -> Result<()> {
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

    pub(crate) fn dispose(&mut self, frame: ApngFrameState) {
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

    pub(crate) fn restart(&mut self) -> Result<()> {
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
