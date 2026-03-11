use std::sync::Arc;

/// Video codec type (matching video-decoder's supported codecs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodec {
    Vp9,
    Av1,
}

/// Pixel format of decoded video frames
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// Planar YUV 4:2:0 (3 separate planes: Y, U, V)
    I420,
    /// Semi-planar YUV 4:2:0 (2 planes: Y, interleaved UV)
    Nv12,
    /// Packed RGBA 8-bit (single plane)
    Rgba,
    /// Packed BGRA 8-bit (single plane)
    Bgra,
}

/// A decoded video frame with owned plane data.
///
/// For I420: planes[0]=Y, planes[1]=U, planes[2]=V (3 planes)
/// For NV12: planes[0]=Y, planes[1]=UV (2 planes, planes[2] is empty)
/// For RGBA/BGRA: planes[0]=packed RGBA/BGRA pixels, planes[1..]=empty
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    pub planes: [Vec<u8>; 3],
    pub strides: [u32; 3],
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    /// Presentation timestamp in microseconds
    pub pts_us: i64,
}

impl DecodedFrame {
    pub fn new_empty() -> Self {
        Self {
            planes: [Vec::new(), Vec::new(), Vec::new()],
            strides: [0; 3],
            width: 0,
            height: 0,
            format: PixelFormat::I420,
            pts_us: 0,
        }
    }
}

/// Status returned by decoder operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeStatus {
    Ok,
    /// Decoder needs more data before it can produce output
    Again,
    /// End of stream
    Eof,
}

/// Playback state of the video player
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Idle,
    Loading,
    Playing,
    Paused,
    Stopped,
    Ended,
    Error,
}

/// Shared decoded frame for renderer consumption
pub type SharedFrame = Arc<DecodedFrame>;
