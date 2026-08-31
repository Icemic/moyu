use std::time::Duration;

use crate::codecs::{ApngDecoder, JxlAnimationDecoder, WebPAnimationDecoder};
use crate::error::Result;
use crate::ops;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationFormat {
    Apng,
    WebP,
    Jxl,
}

pub struct AnimationDecoder {
    inner: AnimationDecoderInner,
}

impl std::fmt::Debug for AnimationDecoder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AnimationDecoder")
            .field("width", &self.width())
            .field("height", &self.height())
            .finish_non_exhaustive()
    }
}

enum AnimationDecoderInner {
    Apng(ApngDecoder),
    WebP(WebPAnimationDecoder),
    Jxl(JxlAnimationDecoder),
}

impl AnimationDecoder {
    pub fn new(data: Vec<u8>, format: AnimationFormat) -> Result<Self> {
        let inner = match format {
            AnimationFormat::Apng => AnimationDecoderInner::Apng(ApngDecoder::new(data)?),
            AnimationFormat::WebP => AnimationDecoderInner::WebP(WebPAnimationDecoder::new(data)?),
            AnimationFormat::Jxl => AnimationDecoderInner::Jxl(JxlAnimationDecoder::new(data)?),
        };
        Ok(Self { inner })
    }

    pub fn width(&self) -> u32 {
        match &self.inner {
            AnimationDecoderInner::Apng(decoder) => decoder.width,
            AnimationDecoderInner::WebP(decoder) => decoder.width,
            AnimationDecoderInner::Jxl(decoder) => decoder.width,
        }
    }

    pub fn height(&self) -> u32 {
        match &self.inner {
            AnimationDecoderInner::Apng(decoder) => decoder.height,
            AnimationDecoderInner::WebP(decoder) => decoder.height,
            AnimationDecoderInner::Jxl(decoder) => decoder.height,
        }
    }

    pub fn advance(&mut self) -> Result<Option<Duration>> {
        match &mut self.inner {
            AnimationDecoderInner::Apng(decoder) => decoder.advance(),
            AnimationDecoderInner::WebP(decoder) => decoder.advance(),
            AnimationDecoderInner::Jxl(decoder) => decoder.advance(),
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
            AnimationDecoderInner::Jxl(decoder) => {
                ops::copy_premultiplied_rgba8(&decoder.canvas, output)
            }
        }
        Ok(())
    }

    pub fn restart(&mut self) -> Result<()> {
        match &mut self.inner {
            AnimationDecoderInner::Apng(decoder) => decoder.restart(),
            AnimationDecoderInner::WebP(decoder) => decoder.restart(),
            AnimationDecoderInner::Jxl(decoder) => decoder.restart(),
        }
    }
}
