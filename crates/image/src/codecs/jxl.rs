use jxl::api::{
    JxlDecoder, JxlDecoderOptions, JxlOutputBuffer, JxlParallelRunner, JxlParallelRunnerFun,
    JxlPixelFormat, ProcessingResult, check_signature, states,
};

use crate::Rgba8Image;
use crate::codecs::{MAX_DECODE_BYTES, rgba8_len, rgba8_stride};
use crate::error::{ImageError, Result};

const MAX_JXL_SAMPLES: usize = MAX_DECODE_BYTES;

pub(crate) fn is_jxl(data: &[u8]) -> bool {
    matches!(
        check_signature(data),
        ProcessingResult::Complete { result: Some(_) }
    )
}

pub(crate) fn decode(data: &[u8]) -> Result<Rgba8Image> {
    let mut input = data;
    let decoder = JxlDecoder::<states::Initialized>::new(decoder_options());
    let mut decoder = match decoder.process(&mut input, Some(&mut RayonParallelRunner)) {
        Ok(ProcessingResult::Complete { result }) => result,
        Ok(ProcessingResult::NeedsMoreInput { .. }) => {
            return Err(ImageError::new("truncated JPEG XL image"));
        }
        Err(error) => {
            return Err(ImageError::with_source(
                "failed to read JPEG XL header",
                error,
            ));
        }
    };

    let (width, height) = image_size(decoder.basic_info().size)?;
    let mut pixels = vec![0; rgba8_len(width, height)?];
    decoder.set_pixel_format(JxlPixelFormat::rgba8(
        decoder.basic_info().extra_channels.len(),
    ));

    let frame_decoder = match decoder.process(&mut input, Some(&mut RayonParallelRunner)) {
        Ok(ProcessingResult::Complete { result }) => result,
        Ok(ProcessingResult::NeedsMoreInput { .. }) => {
            return Err(ImageError::new("truncated JPEG XL image"));
        }
        Err(error) => {
            return Err(ImageError::with_source(
                "failed to read JPEG XL frame",
                error,
            ));
        }
    };
    let mut output = [JxlOutputBuffer::new(
        &mut pixels,
        height as usize,
        width as usize * 4,
    )];
    match frame_decoder.process(&mut input, &mut output, Some(&mut RayonParallelRunner)) {
        Ok(ProcessingResult::Complete { .. }) => {
            Rgba8Image::from_rgba8(width, height, rgba8_stride(width)?, pixels)
        }
        Ok(ProcessingResult::NeedsMoreInput { .. }) => {
            Err(ImageError::new("truncated JPEG XL image"))
        }
        Err(error) => Err(ImageError::with_source(
            "failed to decode JPEG XL frame",
            error,
        )),
    }
}

pub(crate) struct JxlAnimationDecoder {
    data: Vec<u8>,
    decoder: Option<JxlDecoder<states::WithImageInfo>>,
    input_offset: usize,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) canvas: Vec<u8>,
}

impl JxlAnimationDecoder {
    pub(crate) fn new(data: Vec<u8>) -> Result<Self> {
        let (decoder, input_offset, width, height) = create_animation_decoder(&data)?;
        let canvas = vec![0; rgba8_len(width, height)?];
        Ok(Self {
            data,
            decoder: Some(decoder),
            input_offset,
            width,
            height,
            canvas,
        })
    }

    pub(crate) fn advance(&mut self) -> Result<Option<std::time::Duration>> {
        loop {
            let Some(decoder) = self.decoder.take() else {
                return Ok(None);
            };
            let mut input = &self.data[self.input_offset..];
            let frame_decoder = match decoder.process(&mut input, Some(&mut RayonParallelRunner)) {
                Ok(ProcessingResult::Complete { result }) => result,
                Ok(ProcessingResult::NeedsMoreInput { .. }) => {
                    return Err(ImageError::new("truncated JPEG XL animation"));
                }
                Err(error) => {
                    return Err(ImageError::with_source(
                        "failed to read JPEG XL animation frame",
                        error,
                    ));
                }
            };
            self.input_offset = self.data.len() - input.len();
            let duration = frame_decoder.frame_header().duration.unwrap_or_default();
            let mut output = [JxlOutputBuffer::new(
                &mut self.canvas,
                self.height as usize,
                self.width as usize * 4,
            )];
            let mut input = &self.data[self.input_offset..];
            let decoder = match frame_decoder.process(
                &mut input,
                &mut output,
                Some(&mut RayonParallelRunner),
            ) {
                Ok(ProcessingResult::Complete { result }) => result,
                Ok(ProcessingResult::NeedsMoreInput { .. }) => {
                    return Err(ImageError::new("truncated JPEG XL animation"));
                }
                Err(error) => {
                    return Err(ImageError::with_source(
                        "failed to decode JPEG XL animation frame",
                        error,
                    ));
                }
            };
            self.input_offset = self.data.len() - input.len();
            let has_more_frames = decoder.has_more_frames();
            self.decoder = has_more_frames.then_some(decoder);

            if duration.is_finite() && duration > 0.0 {
                return Ok(Some(std::time::Duration::from_secs_f64(duration / 1_000.0)));
            }
            if !has_more_frames {
                return Ok(None);
            }
        }
    }

    pub(crate) fn restart(&mut self) -> Result<()> {
        let (decoder, input_offset, width, height) = create_animation_decoder(&self.data)?;
        self.decoder = Some(decoder);
        self.input_offset = input_offset;
        self.width = width;
        self.height = height;
        self.canvas = vec![0; rgba8_len(width, height)?];
        Ok(())
    }
}

fn decoder_options() -> JxlDecoderOptions {
    let mut options = JxlDecoderOptions::default();
    options.coalescing = true;
    options.sample_limit = Some(MAX_JXL_SAMPLES);
    options.premultiply_output = false;
    options
}

fn image_size(size: (usize, usize)) -> Result<(u32, u32)> {
    let width = u32::try_from(size.0).map_err(|_| ImageError::invalid_layout())?;
    let height = u32::try_from(size.1).map_err(|_| ImageError::invalid_layout())?;
    rgba8_len(width, height)?;
    Ok((width, height))
}

struct RayonParallelRunner;

impl JxlParallelRunner for RayonParallelRunner {
    #[cfg(not(target_arch = "wasm32"))]
    fn run(&mut self, num: usize, fun: &JxlParallelRunnerFun<'_>) -> jxl::error::Result<()> {
        use rayon::prelude::*;

        if num == 1 || rayon::current_num_threads() == 1 {
            for index in 0..num {
                fun(index)?;
            }
            return Ok(());
        }

        (0..num).into_par_iter().try_for_each(fun)
    }

    #[cfg(target_arch = "wasm32")]
    fn run(&mut self, num: usize, fun: &JxlParallelRunnerFun<'_>) -> jxl::error::Result<()> {
        for index in 0..num {
            fun(index)?;
        }
        Ok(())
    }
}

fn create_animation_decoder(
    data: &[u8],
) -> Result<(JxlDecoder<states::WithImageInfo>, usize, u32, u32)> {
    let mut input = data;
    let decoder = JxlDecoder::<states::Initialized>::new(decoder_options());
    let mut decoder = match decoder.process(&mut input, Some(&mut RayonParallelRunner)) {
        Ok(ProcessingResult::Complete { result }) => result,
        Ok(ProcessingResult::NeedsMoreInput { .. }) => {
            return Err(ImageError::new("truncated JPEG XL animation"));
        }
        Err(error) => {
            return Err(ImageError::with_source(
                "failed to read JPEG XL animation header",
                error,
            ));
        }
    };
    if decoder.basic_info().animation.is_none() {
        return Err(ImageError::new("JPEG XL image is not animated"));
    }
    let (width, height) = image_size(decoder.basic_info().size)?;
    decoder.set_pixel_format(JxlPixelFormat::rgba8(
        decoder.basic_info().extra_channels.len(),
    ));
    Ok((decoder, data.len() - input.len(), width, height))
}
