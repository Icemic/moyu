use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;

use anyhow::{Result, anyhow};
use js_sys::{Array, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    DomRectInit, EncodedVideoChunk, EncodedVideoChunkInit, EncodedVideoChunkType, PlaneLayout,
    VideoDecoder as WasmVideoDecoder, VideoDecoderConfig, VideoDecoderInit, VideoFrame,
    VideoFrameCopyToOptions,
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
    eof_sent: bool,
}

// Safety: wasm32-unknown-unknown is single-threaded; web_sys types are not
// `Send` but there is only one thread, so the trait bound is harmless.
unsafe impl Send for WebDecoder {}

impl WebDecoder {
    fn configure_decoder(decoder: &WasmVideoDecoder, codec_string: &'static str) -> Result<()> {
        let config = VideoDecoderConfig::new(codec_string);
        decoder
            .configure(&config)
            .map_err(|e| anyhow!("Failed to configure WebCodecs VideoDecoder: {:?}", e))
    }

    pub fn new(codec: VideoCodec, _thread_count: i32) -> Result<Self> {
        let (frame_tx, frame_rx) = mpsc::channel::<DecodedFrame>();
        let flush_completed = Rc::new(Cell::new(true));

        let tx_clone = frame_tx.clone();
        let output_callback = Closure::wrap(Box::new(move |frame: VideoFrame| {
            let tx = tx_clone.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(decoded) = extract_frame_data(&frame).await {
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
        let init = EncodedVideoChunkInit::new(&js_data, pts as f64, chunk_type);

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
        // Drain any pending frames
        while self.frame_rx.try_recv().is_ok() {}
    }
}

/// Extract pixel data from a WebCodecs VideoFrame.
async fn extract_frame_data(frame: &VideoFrame) -> Option<DecodedFrame> {
    let timestamp = frame.timestamp().unwrap_or(0.0) as i64;

    let copied = match copy_frame_data(frame, false).await {
        Ok(copied) => copied,
        Err(err) => {
            log::warn!(
                "WebCodecs copyTo(visibleRect) failed: {:?}; frame_format={:?}, coded={}x{}, display={}x{}, visible={:?}",
                err,
                frame.format(),
                frame.coded_width(),
                frame.coded_height(),
                frame.display_width(),
                frame.display_height(),
                frame
                    .visible_rect()
                    .map(|rect| (rect.x(), rect.y(), rect.width(), rect.height()))
            );

            match copy_frame_data(frame, true).await {
                Ok(copied) => copied,
                Err(fallback_err) => {
                    log::warn!(
                        "WebCodecs copyTo(codedRect) also failed: {:?}; frame_format={:?}, coded={}x{}",
                        fallback_err,
                        frame.format(),
                        frame.coded_width(),
                        frame.coded_height()
                    );
                    return None;
                }
            }
        }
    };

    Some(DecodedFrame {
        planes: [copied.planes[0].clone(), Vec::new(), Vec::new()],
        strides: [copied.strides[0], 0, 0],
        width: copied.width,
        height: copied.height,
        format: copied.format,
        pts_us: timestamp,
    })
}

struct CopiedFrameData {
    planes: [Vec<u8>; 3],
    strides: [u32; 3],
    width: u32,
    height: u32,
    format: PixelFormat,
}

#[derive(Clone, Copy)]
enum CopyLayoutMode {
    Explicit,
    Returned,
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
    }
}

fn copy_to_format(pixel_format: PixelFormat) -> &'static str {
    match pixel_format {
        PixelFormat::I420 => "I420",
        PixelFormat::Nv12 => "NV12",
        PixelFormat::Rgba => "RGBA",
        PixelFormat::Bgra => "BGRA",
    }
}

fn plane_layout_array(layouts: [PlaneLayoutInfo; 3], pixel_format: PixelFormat) -> Array {
    let result = Array::new();
    result.push(&PlaneLayout::new(
        layouts[0].offset as u32,
        layouts[0].stride,
    ));

    if matches!(pixel_format, PixelFormat::I420 | PixelFormat::Nv12) {
        result.push(&PlaneLayout::new(
            layouts[1].offset as u32,
            layouts[1].stride,
        ));
    }

    if pixel_format == PixelFormat::I420 {
        result.push(&PlaneLayout::new(
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

async fn copy_frame_data(
    frame: &VideoFrame,
    use_coded_rect: bool,
) -> std::result::Result<CopiedFrameData, JsValue> {
    let (width, height, base_options) = if use_coded_rect {
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

    let strategies = [PixelFormat::Rgba, PixelFormat::Bgra];

    let mut last_error = None;

    for pixel_format in strategies {
        let compact_layouts = compact_plane_layouts(pixel_format, width, height);
        for layout_mode in [CopyLayoutMode::Explicit, CopyLayoutMode::Returned] {
            let options = base_options.clone().unwrap_or_default();
            options.set_format(copy_to_format(pixel_format));
            if matches!(layout_mode, CopyLayoutMode::Explicit) {
                let layout_array = plane_layout_array(compact_layouts, pixel_format);
                options.set_layout(layout_array.as_ref());
            }

            match copy_frame_data_with_options(
                frame,
                pixel_format,
                width,
                height,
                &options,
                layout_mode,
                compact_layouts,
            )
            .await
            {
                Ok(copied) => return Ok(copied),
                Err(err) => {
                    last_error = Some(err);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| JsValue::from_str("VideoFrame copyTo failed")))
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
    }

    Ok(CopiedFrameData {
        planes,
        strides,
        width,
        height,
        format: pixel_format,
    })
}
