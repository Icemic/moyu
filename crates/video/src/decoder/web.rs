use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;

use anyhow::{Result, anyhow};
use js_sys::{Array, Date, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    DomRectInit, EncodedVideoChunk, EncodedVideoChunkInit, EncodedVideoChunkType,
    HardwareAcceleration, PlaneLayout, VideoDecoder as WasmVideoDecoder, VideoDecoderConfig,
    VideoDecoderInit, VideoFrame, VideoFrameCopyToOptions, VideoPixelFormat,
};

use crate::types::{DecodeStatus, DecodedFrame, PixelFormat, VideoCodec};

use super::VideoDecoder;

/// Web-based video decoder using the WebCodecs API.
pub struct WebDecoder {
    decoder: WasmVideoDecoder,
    codec_string: &'static str,
    /// Channel to receive decoded frames from the WebCodecs callback
    frame_rx: mpsc::Receiver<DecodedFrame>,
    /// Keep sender alive so channel doesn't close prematurely
    _frame_tx: mpsc::Sender<DecodedFrame>,
    /// WebCodecs requires a key chunk after configure() and flush().
    needs_key_chunk: bool,
    /// flush() resolves asynchronously, so EOF cannot be reported until it completes.
    flush_completed: Rc<Cell<bool>>,
    /// Cache the first successful WebCodecs copy strategy and reuse it for subsequent frames.
    copy_strategy: Rc<RefCell<Option<CachedCopyStrategy>>>,
    use_external_frames: bool,
    eof_sent: bool,
}

// Safety: wasm32-unknown-unknown is single-threaded; web_sys types are not
// `Send` but there is only one thread, so the trait bound is harmless.
unsafe impl Send for WebDecoder {}

impl WebDecoder {
    fn configure_decoder(decoder: &WasmVideoDecoder, codec_string: &'static str) -> Result<()> {
        let config = VideoDecoderConfig::new(codec_string);
        if is_firefox() {
            config.set_optimize_for_latency(true);
            config.set_hardware_acceleration(HardwareAcceleration::PreferSoftware);
        } else {
            config.set_optimize_for_latency(false);
            config.set_hardware_acceleration(HardwareAcceleration::PreferHardware);
        }
        decoder
            .configure(&config)
            .map_err(|e| anyhow!("Failed to configure WebCodecs VideoDecoder: {:?}", e))
    }

    pub fn new(codec: VideoCodec, _thread_count: i32) -> Result<Self> {
        let (frame_tx, frame_rx) = mpsc::channel::<DecodedFrame>();
        let flush_completed = Rc::new(Cell::new(true));
        let copy_strategy = Rc::new(RefCell::new(None));
        let use_external_frames = should_use_external_frames();

        let tx_clone = frame_tx.clone();
        let copy_strategy_clone = copy_strategy.clone();
        let output_callback = Closure::wrap(Box::new(move |frame: VideoFrame| {
            let tx = tx_clone.clone();
            let copy_strategy = copy_strategy_clone.clone();

            if use_external_frames {
                let decoded = extract_external_frame(frame);
                let _ = tx.send(decoded);
                return;
            }

            wasm_bindgen_futures::spawn_local(async move {
                if let Some(decoded) = extract_frame_data(&frame, &copy_strategy).await {
                    let _ = tx.send(decoded);
                }
                frame.close();
            });
        }) as Box<dyn FnMut(VideoFrame)>);

        let error_callback = Closure::wrap(Box::new(move |err: JsValue| {
            log::error!("WebCodecs VideoDecoder error: {:?}", err);
        }) as Box<dyn FnMut(JsValue)>);

        let init = VideoDecoderInit::new(
            error_callback.as_ref().unchecked_ref(),
            output_callback.as_ref().unchecked_ref(),
        );

        let decoder = WasmVideoDecoder::new(&init)
            .map_err(|e| anyhow!("Failed to create WebCodecs VideoDecoder: {:?}", e))?;

        let codec_string = match codec {
            VideoCodec::Vp9 => "vp09.00.10.08",
            VideoCodec::Av1 => "av01.0.01M.08",
        };

        Self::configure_decoder(&decoder, codec_string)?;

        // Leak closures so they live for the lifetime of the decoder.
        // This is standard practice for wasm_bindgen closures used as callbacks.
        output_callback.forget();
        error_callback.forget();

        Ok(Self {
            decoder,
            codec_string,
            frame_rx,
            _frame_tx: frame_tx,
            needs_key_chunk: true,
            flush_completed,
            copy_strategy,
            use_external_frames,
            eof_sent: false,
        })
    }
}

impl VideoDecoder for WebDecoder {
    fn send_packet(&mut self, data: &[u8], pts: i64) -> Result<DecodeStatus> {
        if data.is_empty() {
            // Flush is asynchronous on WebCodecs. Keep returning Again until the
            // promise resolves and all queued callbacks have had a chance to run.
            self.flush_completed.set(false);
            let flush_completed = self.flush_completed.clone();
            let promise = self.decoder.flush();
            wasm_bindgen_futures::spawn_local(async move {
                match JsFuture::from(promise).await {
                    Ok(_) => flush_completed.set(true),
                    Err(err) => {
                        log::error!("WebCodecs VideoDecoder flush failed: {:?}", err);
                        flush_completed.set(true);
                    }
                }
            });
            self.eof_sent = true;
            self.needs_key_chunk = true;
            return Ok(DecodeStatus::Eof);
        }

        let js_data = Uint8Array::from(data);
        let chunk_type = if self.needs_key_chunk {
            EncodedVideoChunkType::Key
        } else {
            EncodedVideoChunkType::Delta
        };

        // WebCodecs expects timestamp in microseconds.
        // After configure() / flush(), the first successfully queued packet must be a key chunk.
        let init = EncodedVideoChunkInit::new(&js_data, pts as i32, chunk_type);

        let chunk = EncodedVideoChunk::new(&init)
            .map_err(|e| anyhow!("Failed to create EncodedVideoChunk: {:?}", e))?;

        self.decoder.decode(&chunk).map_err(|e| {
            anyhow!(
                "WebCodecs decode() failed: {:?}, queue_size={}, pts_us={}, type={:?}",
                e,
                self.decoder.decode_queue_size(),
                pts,
                chunk_type
            )
        })?;

        if self.needs_key_chunk {
            self.needs_key_chunk = false;
        }

        Ok(DecodeStatus::Ok)
    }

    fn receive_frame(&mut self) -> Result<(DecodeStatus, Option<DecodedFrame>)> {
        match self.frame_rx.try_recv() {
            Ok(frame) => Ok((DecodeStatus::Ok, Some(frame))),
            Err(mpsc::TryRecvError::Empty) => {
                if self.eof_sent
                    && self.flush_completed.get()
                    && self.decoder.decode_queue_size() == 0
                {
                    Ok((DecodeStatus::Eof, None))
                } else {
                    Ok((DecodeStatus::Again, None))
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => Ok((DecodeStatus::Eof, None)),
        }
    }

    fn flush(&mut self) {
        if let Err(err) = self.decoder.reset() {
            log::warn!("WebCodecs VideoDecoder reset failed: {:?}", err);
        }

        if let Err(err) = Self::configure_decoder(&self.decoder, self.codec_string) {
            log::error!(
                "Failed to reconfigure WebCodecs VideoDecoder after reset: {}",
                err
            );
        }

        self.eof_sent = false;
        self.needs_key_chunk = true;
        self.flush_completed.set(true);
        if !self.use_external_frames {
            *self.copy_strategy.borrow_mut() = None;
        }
        // Drain any pending frames
        while self.frame_rx.try_recv().is_ok() {}
    }
}

fn is_firefox() -> bool {
    web_sys::window()
        .map(|window| window.navigator())
        .and_then(|navigator| navigator.user_agent().ok())
        .map(|ua| ua.contains("Firefox/"))
        .unwrap_or(false)
}

fn should_use_external_frames() -> bool {
    if is_firefox() {
        log::info!("WebCodecs external frame upload disabled for Firefox; falling back to copyTo");
        return false;
    }

    true
}

fn extract_external_frame(frame: VideoFrame) -> DecodedFrame {
    let extract_start_ms = perf_now_ms();
    let timestamp = frame.timestamp().unwrap_or(0.0) as i64;
    let width = frame.display_width();
    let height = frame.display_height();

    DecodedFrame::with_external_frame(frame, width, height, timestamp)
}

/// Extract pixel data from a WebCodecs VideoFrame.
async fn extract_frame_data(
    frame: &VideoFrame,
    copy_strategy: &RefCell<Option<CachedCopyStrategy>>,
) -> Option<DecodedFrame> {
    let extract_start_ms = perf_now_ms();
    let timestamp = frame.timestamp().unwrap_or(0.0) as i64;

    let cached_strategy = *copy_strategy.borrow();
    let (copied, profile_path, profile_attempts, profile_copy_ms, profile_strategy) = if let Some(
        strategy,
    ) =
        cached_strategy
    {
        match copy_frame_data_with_strategy(frame, strategy).await {
            Ok(result) => (result.copied, "cached", 1, result.copy_ms, result.strategy),
            Err(err) => {
                log::debug!(
                    "Cached WebCodecs copy strategy failed, retrying probe: {:?}; strategy={:?}; frame_format={:?}",
                    err,
                    strategy,
                    frame.format()
                );
                *copy_strategy.borrow_mut() = None;

                match probe_copy_frame_data(frame).await {
                    Ok(result) => {
                        *copy_strategy.borrow_mut() = Some(result.strategy);
                        (
                            result.copied,
                            "reprobe",
                            result.attempts,
                            result.copy_ms,
                            result.strategy,
                        )
                    }
                    Err(probe_err) => {
                        log::warn!(
                            "WebCodecs copyTo failed after retry: {:?}; frame_format={:?}, coded={}x{}, display={}x{}, visible={:?}",
                            probe_err,
                            frame.format(),
                            frame.coded_width(),
                            frame.coded_height(),
                            frame.display_width(),
                            frame.display_height(),
                            frame.visible_rect().map(|rect| (
                                rect.x(),
                                rect.y(),
                                rect.width(),
                                rect.height()
                            ))
                        );
                        return None;
                    }
                }
            }
        }
    } else {
        match probe_copy_frame_data(frame).await {
            Ok(result) => {
                *copy_strategy.borrow_mut() = Some(result.strategy);
                (
                    result.copied,
                    "probe",
                    result.attempts,
                    result.copy_ms,
                    result.strategy,
                )
            }
            Err(err) => {
                log::warn!(
                    "WebCodecs copyTo probe failed: {:?}; frame_format={:?}, coded={}x{}, display={}x{}, visible={:?}",
                    err,
                    frame.format(),
                    frame.coded_width(),
                    frame.coded_height(),
                    frame.display_width(),
                    frame.display_height(),
                    frame.visible_rect().map(|rect| (
                        rect.x(),
                        rect.y(),
                        rect.width(),
                        rect.height()
                    ))
                );
                return None;
            }
        }
    };

    let extract_total_ms = perf_now_ms() - extract_start_ms;

    let CopiedFrameData {
        planes,
        strides,
        width,
        height,
        format,
    } = copied;

    log::debug!(
        "WebCodecs frame profile: pts_us={}, {}x{}, format={:?}, path={}, strategy={:?}, attempts={}, copy_ms={:.3}, total_ms={:.3}",
        timestamp,
        width,
        height,
        format,
        profile_path,
        profile_strategy,
        profile_attempts,
        profile_copy_ms,
        extract_total_ms,
    );

    Some(DecodedFrame {
        planes,
        strides,
        width,
        height,
        format,
        pts_us: timestamp,
        external_frame: None,
    })
}

struct CopiedFrameData {
    planes: [Vec<u8>; 3],
    strides: [u32; 3],
    width: u32,
    height: u32,
    format: PixelFormat,
}

#[derive(Clone, Copy, Debug)]
enum CopyLayoutMode {
    Explicit,
    Returned,
}

#[derive(Clone, Copy, Debug)]
struct CachedCopyStrategy {
    pixel_format: PixelFormat,
    layout_mode: CopyLayoutMode,
    use_coded_rect: bool,
}

struct CopyProbeResult {
    copied: CopiedFrameData,
    strategy: CachedCopyStrategy,
    attempts: u32,
    copy_ms: f64,
}

struct CopyExecutionResult {
    copied: CopiedFrameData,
    strategy: CachedCopyStrategy,
    copy_ms: f64,
}

#[derive(Clone, Copy)]
struct PlaneLayoutInfo {
    offset: usize,
    stride: u32,
}

fn compact_plane_layouts(
    pixel_format: PixelFormat,
    width: u32,
    height: u32,
) -> [PlaneLayoutInfo; 3] {
    match pixel_format {
        PixelFormat::I420 => {
            let y_stride = width;
            let uv_stride = width.div_ceil(2);
            let y_bytes = y_stride as usize * height as usize;
            let uv_height = height.div_ceil(2) as usize;
            let u_bytes = uv_stride as usize * uv_height;
            [
                PlaneLayoutInfo {
                    offset: 0,
                    stride: y_stride,
                },
                PlaneLayoutInfo {
                    offset: y_bytes,
                    stride: uv_stride,
                },
                PlaneLayoutInfo {
                    offset: y_bytes + u_bytes,
                    stride: uv_stride,
                },
            ]
        }
        PixelFormat::Nv12 => {
            let y_stride = width;
            let y_bytes = y_stride as usize * height as usize;
            [
                PlaneLayoutInfo {
                    offset: 0,
                    stride: y_stride,
                },
                PlaneLayoutInfo {
                    offset: y_bytes,
                    stride: width,
                },
                PlaneLayoutInfo {
                    offset: y_bytes,
                    stride: 0,
                },
            ]
        }
        PixelFormat::Rgba | PixelFormat::Bgra => [
            PlaneLayoutInfo {
                offset: 0,
                stride: width * 4,
            },
            PlaneLayoutInfo {
                offset: 0,
                stride: 0,
            },
            PlaneLayoutInfo {
                offset: 0,
                stride: 0,
            },
        ],
        PixelFormat::External => unreachable!("external frames do not use copyTo plane layouts"),
    }
}

fn copy_to_format(pixel_format: PixelFormat) -> VideoPixelFormat {
    match pixel_format {
        PixelFormat::I420 => VideoPixelFormat::I420,
        PixelFormat::Nv12 => VideoPixelFormat::Nv12,
        PixelFormat::Rgba => VideoPixelFormat::Rgba,
        PixelFormat::Bgra => VideoPixelFormat::Bgra,
        PixelFormat::External => unreachable!("external frames do not use copyTo format strings"),
    }
}

fn plane_layout_array(
    layouts: [PlaneLayoutInfo; 3],
    pixel_format: PixelFormat,
) -> Vec<PlaneLayout> {
    let mut result = vec![];
    result.push(PlaneLayout::new(
        layouts[0].offset as u32,
        layouts[0].stride,
    ));

    if matches!(pixel_format, PixelFormat::I420 | PixelFormat::Nv12) {
        result.push(PlaneLayout::new(
            layouts[1].offset as u32,
            layouts[1].stride,
        ));
    }

    if pixel_format == PixelFormat::I420 {
        result.push(PlaneLayout::new(
            layouts[2].offset as u32,
            layouts[2].stride,
        ));
    }

    result
}

fn read_plane_layout(layouts: &Array, index: u32) -> Option<PlaneLayoutInfo> {
    let value = layouts.get(index);
    if value.is_undefined() || value.is_null() {
        return None;
    }

    let layout = value.dyn_into::<PlaneLayout>().ok()?;
    Some(PlaneLayoutInfo {
        offset: layout.get_offset() as usize,
        stride: layout.get_stride(),
    })
}

fn slice_plane(
    buffer: &[u8],
    layout: PlaneLayoutInfo,
    plane_height: usize,
) -> std::result::Result<Vec<u8>, JsValue> {
    let byte_len = layout.stride as usize * plane_height;
    let end = layout.offset.saturating_add(byte_len);
    if end > buffer.len() {
        return Err(JsValue::from_str(
            "Plane layout exceeds copied VideoFrame buffer",
        ));
    }

    Ok(buffer[layout.offset..end].to_vec())
}

fn perf_now_ms() -> f64 {
    Date::now()
}

async fn probe_copy_frame_data(
    frame: &VideoFrame,
) -> std::result::Result<CopyProbeResult, JsValue> {
    let strategies = [
        PixelFormat::Nv12,
        PixelFormat::I420,
        PixelFormat::Rgba,
        PixelFormat::Bgra,
    ];

    let mut last_error = None;
    let mut attempts = 0;

    for use_coded_rect in [false, true] {
        for pixel_format in strategies {
            for layout_mode in [CopyLayoutMode::Explicit, CopyLayoutMode::Returned] {
                attempts += 1;
                let strategy = CachedCopyStrategy {
                    pixel_format,
                    layout_mode,
                    use_coded_rect,
                };

                match copy_frame_data_with_strategy(frame, strategy).await {
                    Ok(result) => {
                        return Ok(CopyProbeResult {
                            copied: result.copied,
                            strategy: result.strategy,
                            attempts,
                            copy_ms: result.copy_ms,
                        });
                    }
                    Err(err) => {
                        last_error = Some(err);
                    }
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| JsValue::from_str("VideoFrame copyTo failed")))
}

async fn copy_frame_data_with_strategy(
    frame: &VideoFrame,
    strategy: CachedCopyStrategy,
) -> std::result::Result<CopyExecutionResult, JsValue> {
    let (width, height, base_options) = if strategy.use_coded_rect {
        let coded_rect = frame
            .coded_rect()
            .ok_or_else(|| JsValue::from_str("VideoFrame has no codedRect"))?;
        let rect = DomRectInit::new();
        rect.set_x(coded_rect.x());
        rect.set_y(coded_rect.y());
        rect.set_width(coded_rect.width());
        rect.set_height(coded_rect.height());

        let options = VideoFrameCopyToOptions::new();
        options.set_rect(&rect);
        (frame.coded_width(), frame.coded_height(), Some(options))
    } else {
        let visible_rect = frame.visible_rect();
        let width = visible_rect
            .as_ref()
            .map(|rect| rect.width() as u32)
            .unwrap_or(frame.coded_width());
        let height = visible_rect
            .as_ref()
            .map(|rect| rect.height() as u32)
            .unwrap_or(frame.coded_height());

        (width, height, Some(VideoFrameCopyToOptions::new()))
    };

    let compact_layouts = compact_plane_layouts(strategy.pixel_format, width, height);
    let options = base_options.clone().unwrap_or_default();
    options.set_format(copy_to_format(strategy.pixel_format));
    if matches!(strategy.layout_mode, CopyLayoutMode::Explicit) {
        let layout_array = plane_layout_array(compact_layouts, strategy.pixel_format);
        options.set_layout(layout_array.as_ref());
    }

    let copy_start_ms = perf_now_ms();
    let copied = copy_frame_data_with_options(
        frame,
        strategy.pixel_format,
        width,
        height,
        &options,
        strategy.layout_mode,
        compact_layouts,
    )
    .await?;

    Ok(CopyExecutionResult {
        copied,
        strategy,
        copy_ms: perf_now_ms() - copy_start_ms,
    })
}

async fn copy_frame_data_with_options(
    frame: &VideoFrame,
    pixel_format: PixelFormat,
    width: u32,
    height: u32,
    options: &VideoFrameCopyToOptions,
    layout_mode: CopyLayoutMode,
    fallback_layouts: [PlaneLayoutInfo; 3],
) -> std::result::Result<CopiedFrameData, JsValue> {
    let allocation_size = frame.allocation_size_with_options(options)?;

    if allocation_size == 0 {
        return Err(JsValue::from_str("VideoFrame allocationSize returned 0"));
    }

    let mut buffer = vec![0u8; allocation_size as usize];
    let js_buf = Uint8Array::new_with_length(allocation_size);
    let promise = frame.copy_to_with_buffer_source_and_options(&js_buf.buffer(), options);

    let layouts_value = JsFuture::from(promise).await?;
    js_buf.copy_to(&mut buffer);

    let mut planes = [Vec::new(), Vec::new(), Vec::new()];
    let mut strides = [0u32; 3];
    let returned_layouts = match layout_mode {
        CopyLayoutMode::Explicit => None,
        CopyLayoutMode::Returned => Some(layouts_value.dyn_into::<Array>()?),
    };

    match pixel_format {
        PixelFormat::I420 => {
            let plane_widths = [width, width.div_ceil(2), width.div_ceil(2)];
            let plane_heights = [height, height.div_ceil(2), height.div_ceil(2)];

            for plane_index in 0..3 {
                let layout = returned_layouts
                    .as_ref()
                    .and_then(|layouts| read_plane_layout(layouts, plane_index as u32))
                    .unwrap_or(fallback_layouts[plane_index]);
                let stride = layout.stride;
                let plane_height = plane_heights[plane_index] as usize;
                planes[plane_index] = slice_plane(&buffer, layout, plane_height)?;
                strides[plane_index] = stride.max(plane_widths[plane_index]);
            }
        }
        PixelFormat::Nv12 => {
            let y_layout = returned_layouts
                .as_ref()
                .and_then(|layouts| read_plane_layout(layouts, 0))
                .unwrap_or(fallback_layouts[0]);
            let uv_layout = returned_layouts
                .as_ref()
                .and_then(|layouts| read_plane_layout(layouts, 1))
                .unwrap_or(fallback_layouts[1]);

            let y_stride = y_layout.stride;
            planes[0] = slice_plane(&buffer, y_layout, height as usize)?;
            strides[0] = y_stride.max(width);

            let uv_stride = uv_layout.stride;
            let uv_height = height.div_ceil(2) as usize;
            planes[1] = slice_plane(&buffer, uv_layout, uv_height)?;
            strides[1] = uv_stride;
        }
        PixelFormat::Rgba | PixelFormat::Bgra => {
            let layout = returned_layouts
                .as_ref()
                .and_then(|layouts| read_plane_layout(layouts, 0))
                .unwrap_or(fallback_layouts[0]);

            let stride = layout.stride;
            planes[0] = slice_plane(&buffer, layout, height as usize)?;
            strides[0] = stride.max(width * 4);
        }
        PixelFormat::External => unreachable!("external frames do not use copied planes"),
    }

    Ok(CopiedFrameData {
        planes,
        strides,
        width,
        height,
        format: pixel_format,
    })
}
