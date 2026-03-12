use std::sync::Arc;

#[cfg(web)]
use web_sys::VideoFrame;

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
    /// Web-only opaque external frame handle for zero-copy GPU upload paths.
    External,
}

/// A decoded video frame with owned plane data.
///
/// For I420: planes[0]=Y, planes[1]=U, planes[2]=V (3 planes)
/// For NV12: planes[0]=Y, planes[1]=UV (2 planes, planes[2] is empty)
/// For RGBA/BGRA: planes[0]=packed RGBA/BGRA pixels, planes[1..]=empty
#[derive(Clone)]
pub struct DecodedFrame {
    pub planes: [Vec<u8>; 3],
    pub strides: [u32; 3],
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    /// Presentation timestamp in microseconds
    pub pts_us: i64,
    #[cfg(web)]
    pub external_frame: Option<VideoFrame>,
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
            #[cfg(web)]
            external_frame: None,
        }
    }

    #[cfg(web)]
    pub fn with_external_frame(frame: VideoFrame, width: u32, height: u32, pts_us: i64) -> Self {
        Self {
            planes: [Vec::new(), Vec::new(), Vec::new()],
            strides: [0; 3],
            width,
            height,
            format: PixelFormat::External,
            pts_us,
            external_frame: Some(frame),
        }
    }

    #[cfg(web)]
    pub fn external_frame(&self) -> Option<&VideoFrame> {
        self.external_frame.as_ref()
    }

    pub fn is_external(&self) -> bool {
        self.format == PixelFormat::External
    }
}

impl std::fmt::Debug for DecodedFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodedFrame")
            .field("strides", &self.strides)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("format", &self.format)
            .field("pts_us", &self.pts_us)
            .finish_non_exhaustive()
    }
}

// wasm32-unknown-unknown runs the video path on a single thread; carrying the
// opaque VideoFrame handle through Arc-backed player state is safe under that model.
#[cfg(web)]
unsafe impl Send for DecodedFrame {}

#[cfg(web)]
unsafe impl Sync for DecodedFrame {}

#[cfg(web)]
impl Drop for DecodedFrame {
    fn drop(&mut self) {
        if let Some(frame) = self.external_frame.take() {
            frame.close();
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
