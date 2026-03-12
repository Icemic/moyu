use std::sync::OnceLock;

use anyhow::{Result, anyhow};
use libloading::{Library, Symbol};

use crate::types::{DecodeStatus, DecodedFrame, PixelFormat, VideoCodec};

use super::VideoDecoder;

// ── C type mirrors (matching video-decoder's ABI) ─────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoyuVideoCodec {
    Vp9 = 0,
    Av1 = 1,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum MoyuVideoPixelFormat {
    I420 = 0,
    Nv12 = 1,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum MoyuVideoResult {
    Ok = 0,
    Error = -1,
    Again = -2,
    Eof = -3,
    InvalidArgument = -4,
}

#[repr(C)]
struct MoyuVideoDecoderConfig {
    codec: MoyuVideoCodec,
    thread_count: i32,
}

#[repr(C)]
struct MoyuVideoFrame {
    planes: [*const u8; 3],
    strides: [i32; 3],
    width: i32,
    height: i32,
    format: MoyuVideoPixelFormat,
    pts: i64,
}

impl Default for MoyuVideoFrame {
    fn default() -> Self {
        Self {
            planes: [std::ptr::null(); 3],
            strides: [0; 3],
            width: 0,
            height: 0,
            format: MoyuVideoPixelFormat::I420,
            pts: 0,
        }
    }
}

// Opaque handle type
enum MoyuVideoDecoderHandle {}

// ── Function pointer types ────────────────────────────────────────────────────

type FnDecoderCreate = unsafe extern "C" fn(
    *const MoyuVideoDecoderConfig,
    *mut *mut MoyuVideoDecoderHandle,
) -> MoyuVideoResult;

type FnDecoderSendPacket =
    unsafe extern "C" fn(*mut MoyuVideoDecoderHandle, *const u8, i32, i64) -> MoyuVideoResult;

type FnDecoderReceiveFrame =
    unsafe extern "C" fn(*mut MoyuVideoDecoderHandle, *mut MoyuVideoFrame) -> MoyuVideoResult;

type FnDecoderFlush = unsafe extern "C" fn(*mut MoyuVideoDecoderHandle);
type FnDecoderDestroy = unsafe extern "C" fn(*mut MoyuVideoDecoderHandle);

// ── Loaded library wrapper ────────────────────────────────────────────────────

struct WrapperLib {
    _lib: Library,
    create: FnDecoderCreate,
    send_packet: FnDecoderSendPacket,
    receive_frame: FnDecoderReceiveFrame,
    flush: FnDecoderFlush,
    destroy: FnDecoderDestroy,
}

// Safety: The library functions are safe to call from any thread as long as
// the decoder handle is not shared across threads (which we ensure).
unsafe impl Send for WrapperLib {}
unsafe impl Sync for WrapperLib {}

/// Attempt to find and load the video decoder library.
/// Returns None if the library cannot be found (graceful degradation).
fn try_load_library() -> Option<WrapperLib> {
    let lib_path = find_library_path()?;
    log::info!("Loading video decoder library from: {:?}", lib_path);

    unsafe {
        let lib = match Library::new(&lib_path) {
            Ok(lib) => lib,
            Err(e) => {
                log::warn!("Failed to load video decoder library {:?}: {}", lib_path, e);
                return None;
            }
        };

        let create: Symbol<FnDecoderCreate> = match lib.get(b"moyu_video_decoder_create\0") {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Symbol moyu_video_decoder_create not found: {}", e);
                return None;
            }
        };
        let send_packet: Symbol<FnDecoderSendPacket> =
            match lib.get(b"moyu_video_decoder_send_packet\0") {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("Symbol moyu_video_decoder_send_packet not found: {}", e);
                    return None;
                }
            };
        let receive_frame: Symbol<FnDecoderReceiveFrame> =
            match lib.get(b"moyu_video_decoder_receive_frame\0") {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("Symbol moyu_video_decoder_receive_frame not found: {}", e);
                    return None;
                }
            };
        let flush: Symbol<FnDecoderFlush> = match lib.get(b"moyu_video_decoder_flush\0") {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Symbol moyu_video_decoder_flush not found: {}", e);
                return None;
            }
        };
        let destroy: Symbol<FnDecoderDestroy> = match lib.get(b"moyu_video_decoder_destroy\0") {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Symbol moyu_video_decoder_destroy not found: {}", e);
                return None;
            }
        };

        Some(WrapperLib {
            create: *create,
            send_packet: *send_packet,
            receive_frame: *receive_frame,
            flush: *flush,
            destroy: *destroy,
            _lib: lib,
        })
    }
}

fn find_library_path() -> Option<std::ffi::OsString> {
    // Check MOYU_LIB env var first
    if let Ok(p) = std::env::var("MOYU_LIB") {
        if std::path::Path::new(&p).exists() {
            return Some(p.into());
        }
    }

    // Look next to the executable
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;

    #[cfg(target_os = "windows")]
    let name = "moyu_video.dll";
    #[cfg(target_os = "macos")]
    let name = "libmoyu_video.dylib";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let name = "libmoyu_video.so";

    let path = dir.join(name);
    if path.exists() {
        Some(path.into_os_string())
    } else {
        log::info!(
            "Video decoder library not found at {:?}, video playback will be unavailable",
            path
        );
        None
    }
}

static WRAPPER_LIB: OnceLock<Option<WrapperLib>> = OnceLock::new();

fn get_wrapper_lib() -> Option<&'static WrapperLib> {
    WRAPPER_LIB.get_or_init(try_load_library).as_ref()
}

// ── Native decoder ────────────────────────────────────────────────────────────

pub struct NativeDecoder {
    handle: *mut MoyuVideoDecoderHandle,
    lib: &'static WrapperLib,
}

// Safety: NativeDecoder owns the handle exclusively and is not shared.
unsafe impl Send for NativeDecoder {}

impl NativeDecoder {
    pub fn new(codec: VideoCodec, thread_count: i32) -> Result<Self> {
        let lib = get_wrapper_lib()
            .ok_or_else(|| anyhow!("Video decoder library not available. Ensure moyu_video.dll is present next to the executable."))?;

        let c_codec = match codec {
            VideoCodec::Vp9 => MoyuVideoCodec::Vp9,
            VideoCodec::Av1 => MoyuVideoCodec::Av1,
        };

        let config = MoyuVideoDecoderConfig {
            codec: c_codec,
            thread_count,
        };

        let mut handle: *mut MoyuVideoDecoderHandle = std::ptr::null_mut();
        let result = unsafe { (lib.create)(&config, &mut handle) };

        if result != MoyuVideoResult::Ok || handle.is_null() {
            return Err(anyhow!("Failed to create video decoder: {:?}", result));
        }

        Ok(Self { handle, lib })
    }
}

impl VideoDecoder for NativeDecoder {
    fn send_packet(&mut self, data: &[u8], pts: i64) -> Result<DecodeStatus> {
        let result =
            unsafe { (self.lib.send_packet)(self.handle, data.as_ptr(), data.len() as i32, pts) };

        match result {
            MoyuVideoResult::Ok => Ok(DecodeStatus::Ok),
            MoyuVideoResult::Again => Ok(DecodeStatus::Again),
            MoyuVideoResult::Eof => Ok(DecodeStatus::Eof),
            _ => Err(anyhow!("send_packet failed: {:?}", result)),
        }
    }

    fn receive_frame(&mut self) -> Result<(DecodeStatus, Option<DecodedFrame>)> {
        let mut frame = MoyuVideoFrame::default();

        let result = unsafe { (self.lib.receive_frame)(self.handle, &mut frame) };

        match result {
            MoyuVideoResult::Ok => {
                let format = match frame.format {
                    MoyuVideoPixelFormat::I420 => PixelFormat::I420,
                    MoyuVideoPixelFormat::Nv12 => PixelFormat::Nv12,
                };

                let w = frame.width as u32;
                let h = frame.height as u32;

                // Copy plane data (pointers are only valid until next call)
                let planes = match format {
                    PixelFormat::I420 => {
                        let y_size = (frame.strides[0] * frame.height) as usize;
                        let u_size = (frame.strides[1] * (frame.height / 2)) as usize;
                        let v_size = (frame.strides[2] * (frame.height / 2)) as usize;
                        [
                            unsafe { std::slice::from_raw_parts(frame.planes[0], y_size) }.to_vec(),
                            unsafe { std::slice::from_raw_parts(frame.planes[1], u_size) }.to_vec(),
                            unsafe { std::slice::from_raw_parts(frame.planes[2], v_size) }.to_vec(),
                        ]
                    }
                    PixelFormat::Nv12 => {
                        let y_size = (frame.strides[0] * frame.height) as usize;
                        let uv_size = (frame.strides[1] * (frame.height / 2)) as usize;
                        [
                            unsafe { std::slice::from_raw_parts(frame.planes[0], y_size) }.to_vec(),
                            unsafe { std::slice::from_raw_parts(frame.planes[1], uv_size) }
                                .to_vec(),
                            Vec::new(),
                        ]
                    }
                    PixelFormat::Rgba | PixelFormat::Bgra | PixelFormat::External => unreachable!(),
                };

                Ok((
                    DecodeStatus::Ok,
                    Some(DecodedFrame {
                        planes,
                        strides: [
                            frame.strides[0] as u32,
                            frame.strides[1] as u32,
                            frame.strides[2] as u32,
                        ],
                        width: w,
                        height: h,
                        format,
                        pts_us: frame.pts,
                        #[cfg(web)]
                        external_frame: None,
                    }),
                ))
            }
            MoyuVideoResult::Again => Ok((DecodeStatus::Again, None)),
            MoyuVideoResult::Eof => Ok((DecodeStatus::Eof, None)),
            _ => Err(anyhow!("receive_frame failed: {:?}", result)),
        }
    }

    fn flush(&mut self) {
        unsafe { (self.lib.flush)(self.handle) };
    }
}

impl Drop for NativeDecoder {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { (self.lib.destroy)(self.handle) };
            self.handle = std::ptr::null_mut();
        }
    }
}
