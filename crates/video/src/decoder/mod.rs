#[cfg(native)]
mod native;
#[cfg(web)]
mod web;

use anyhow::Result;

use crate::types::{DecodeStatus, DecodedFrame, VideoCodec};

/// Unified video decoder trait for both native (FFmpeg) and web (WebCodecs) backends.
pub trait VideoDecoder: Send {
    /// Send a compressed video packet to the decoder.
    /// `pts` is the presentation timestamp in microseconds.
    fn send_packet(&mut self, data: &[u8], pts: i64) -> Result<DecodeStatus>;

    /// Receive a decoded video frame. Returns `DecodeStatus::Again` if more
    /// packets are needed before a frame can be produced.
    fn receive_frame(&mut self) -> Result<(DecodeStatus, Option<DecodedFrame>)>;

    /// Flush the decoder (e.g. after seeking). Resets internal state.
    fn flush(&mut self);
}

/// Create a platform-appropriate video decoder.
pub fn create_decoder(codec: VideoCodec, thread_count: i32) -> Result<Box<dyn VideoDecoder>> {
    #[cfg(native)]
    {
        native::NativeDecoder::new(codec, thread_count)
            .map(|d| Box::new(d) as Box<dyn VideoDecoder>)
    }
    #[cfg(web)]
    {
        web::WebDecoder::new(codec, thread_count).map(|d| Box::new(d) as Box<dyn VideoDecoder>)
    }
}
