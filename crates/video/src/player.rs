use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Result, anyhow};
use arc_swap::ArcSwapOption;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::conv::IntoSample;

use crate::audio_output::{AudioClock, AudioOutput, AudioRingBuffer};
use crate::decoder::{self, VideoDecoder};
use crate::demuxer::{Demuxer, TrackKind};
use crate::types::*;

/// Maximum number of decoded video frames to buffer
const MAX_VIDEO_BUFFER: usize = 5;
/// Audio ring buffer size (about 2s at 48kHz stereo)
const AUDIO_RING_BUFFER_SIZE: usize = 48000 * 2 * 2;
/// Keep at least this much queued audio before we stop prioritizing audio decode.
const AUDIO_DECODE_LOW_WATERMARK_DIVISOR: usize = 4;
/// Allow spill buffering up to roughly half of the ring buffer before pausing decode.
const AUDIO_SPILL_BUFFER_LIMIT: usize = AUDIO_RING_BUFFER_SIZE / 2;
/// When audio is hungry, allow some extra video packets in flight so demuxing can reach more audio.
const MAX_VIDEO_BUFFER_WHEN_AUDIO_HUNGRY: usize = MAX_VIDEO_BUFFER + 12;
/// Process a larger packet batch while recovering audio to reduce underruns in browsers.
const MAX_PACKET_BATCH_WHEN_AUDIO_HUNGRY: usize = 96;
const MAX_PACKET_BATCH_NORMAL: usize = 10;

/// Video player that orchestrates demuxing, decoding, and A/V synchronization.
pub struct VideoPlayer {
    /// Current playback state
    state: PlaybackState,
    /// Shared latest decoded frame for renderer consumption
    current_frame: Arc<ArcSwapOption<DecodedFrame>>,
    /// Video dimensions
    video_size: Option<(u32, u32)>,
    /// Total duration in seconds
    duration: Option<f64>,
    /// Whether looping is enabled
    loop_enabled: bool,
    /// Current volume (0.0 - 1.0)
    volume: f64,
    /// Whether audio is muted
    muted: bool,

    // Internal decode state (all owned, not shared across threads)
    demuxer: Option<Demuxer>,
    video_decoder: Option<Box<dyn VideoDecoder>>,
    audio_decoder: Option<Box<dyn symphonia::core::codecs::Decoder>>,
    audio_output: Option<AudioOutput>,
    audio_ring_buffer: Option<AudioRingBuffer>,
    audio_resampler: AudioResampler,
    audio_spill_buffer: Vec<f32>,
    audio_clock: AudioClock,

    /// Buffered decoded video frames, sorted by PTS
    video_buffer: VecDeque<Arc<DecodedFrame>>,
    /// Video packets deferred while audio is recovering.
    pending_video_packets: VecDeque<symphonia::core::formats::Packet>,

    /// Video time base (numer/denom) for converting packet timestamps to microseconds
    video_time_base: Option<(u32, u32)>,
    /// Audio time base
    audio_time_base: Option<(u32, u32)>,

    /// Whether we've reached the end of the stream
    eof: AtomicBool,
    /// Whether the demuxer has no more packets to produce.
    demux_exhausted: bool,

    /// Raw file data (kept for re-seeking)
    raw_data: Option<Vec<u8>>,
    /// File extension hint
    file_hint: Option<String>,

    /// Last tick timestamp for fallback clock (when no audio output).
    /// Uses moyu_pal::time::Instant (std on native, web_time on WASM).
    last_tick_time: Option<moyu_pal::time::Instant>,

    /// Device audio sample rate (from cpal output)
    device_sample_rate: u32,
    /// Device audio channel count
    device_channels: u16,
}

impl std::fmt::Debug for VideoPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoPlayer")
            .field("state", &self.state)
            .field("video_size", &self.video_size)
            .field("loop_enabled", &self.loop_enabled)
            .field("volume", &self.volume)
            .field("muted", &self.muted)
            .finish_non_exhaustive()
    }
}

impl VideoPlayer {
    pub fn new() -> Self {
        Self {
            state: PlaybackState::Idle,
            current_frame: Arc::new(ArcSwapOption::empty()),
            video_size: None,
            duration: None,
            loop_enabled: false,
            volume: 1.0,
            muted: false,
            demuxer: None,
            video_decoder: None,
            audio_decoder: None,
            audio_output: None,
            audio_ring_buffer: None,
            audio_resampler: AudioResampler::new(),
            audio_spill_buffer: Vec::new(),
            audio_clock: AudioClock::new(),
            video_buffer: VecDeque::new(),
            pending_video_packets: VecDeque::new(),
            video_time_base: None,
            audio_time_base: None,
            eof: AtomicBool::new(false),
            demux_exhausted: false,
            raw_data: None,
            file_hint: None,
            last_tick_time: None,
            device_sample_rate: 0,
            device_channels: 0,
        }
    }

    /// Load a video file from raw data.
    pub fn load(&mut self, data: Vec<u8>, src_path: Option<&str>) -> Result<()> {
        // Clean up any previous state
        self.stop_internal();

        self.state = PlaybackState::Loading;
        self.file_hint = src_path.map(|s| s.to_string());

        let ext = src_path.and_then(|p| p.rsplit('.').next());

        // Open the demuxer
        // The demuxer will now intelligently probe the file format and first packet
        // to detect whether it's VP9 or AV1, without relying on hints.
        let demuxer = Demuxer::open(data.clone(), ext)?;

        // Get video codec and create video decoder
        let video_codec = demuxer
            .video_codec()
            .ok_or_else(|| anyhow!("No supported video track found (VP9 or AV1)"))?;

        let video_decoder = decoder::create_decoder(video_codec, 0)?;
        // Symphonia doesn't provide video dimensions; they'll be set from the first decoded frame.
        self.video_size = None;
        self.duration = demuxer.duration();
        self.video_time_base = demuxer.video_time_base();
        self.audio_time_base = demuxer.audio_time_base();
        self.pending_video_packets.clear();
        self.demux_exhausted = false;

        // Create audio decoder and output if audio track exists
        if let Some(audio_params) = demuxer.audio_codec_params() {
            let mut params = audio_params.clone();
            // Some containers (e.g. WebM) may not populate channel count in codec params.
            // Provide a stereo default so the decoder can initialize; actual channel
            // count will be read from each decoded AudioBuffer's spec.
            if params.channels.is_none() {
                use symphonia::core::audio::Channels;
                params.channels = Some(Channels::FRONT_LEFT | Channels::FRONT_RIGHT);
            }
            let codecs = moyu_pal::symphonia::get_codec();
            match codecs.make(&params, &Default::default()) {
                Ok(audio_dec) => {
                    self.audio_decoder = Some(audio_dec);

                    let sample_rate = params.sample_rate.unwrap_or(48000);
                    let channels = params.channels.map(|c| c.count() as u16).unwrap_or(2);

                    let ring_buffer = AudioRingBuffer::new(AUDIO_RING_BUFFER_SIZE);
                    let reader = ring_buffer.reader();

                    let clock = self.audio_clock.clone();
                    match AudioOutput::new(sample_rate, channels, reader, clock) {
                        Ok(output) => {
                            self.device_sample_rate = output.sample_rate;
                            self.device_channels = output.channels;
                            self.audio_output = Some(output);
                            self.audio_ring_buffer = Some(ring_buffer);
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to create audio output: {}, video will play without audio",
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to create audio decoder: {}, video will play without audio",
                        e
                    );
                }
            }
        }

        self.demuxer = Some(demuxer);
        self.video_decoder = Some(video_decoder);
        self.raw_data = Some(data);
        self.state = PlaybackState::Stopped;

        Ok(())
    }

    /// Start or resume playback.
    pub fn play(&mut self) -> Result<()> {
        match self.state {
            PlaybackState::Stopped | PlaybackState::Ended => {
                // Reset to beginning
                self.audio_clock.set_position_us(0);
                self.audio_clock.set_playing(true);
                self.eof.store(false, Ordering::Relaxed);

                if let Some(ref mut output) = self.audio_output {
                    output.resume().ok();
                }

                self.last_tick_time = None;

                self.state = PlaybackState::Playing;
                Ok(())
            }
            PlaybackState::Paused => self.resume(),
            PlaybackState::Loading => Err(anyhow!("Video is still loading")),
            PlaybackState::Playing => Ok(()), // already playing
            _ => Err(anyhow!("Cannot play in current state: {:?}", self.state)),
        }
    }

    /// Pause playback.
    pub fn pause(&mut self) -> Result<()> {
        if self.state == PlaybackState::Playing {
            self.audio_clock.set_playing(false);
            if let Some(ref output) = self.audio_output {
                output.pause().ok();
            }
            self.state = PlaybackState::Paused;
        }
        Ok(())
    }

    /// Resume from pause.
    pub fn resume(&mut self) -> Result<()> {
        if self.state == PlaybackState::Paused {
            self.audio_clock.set_playing(true);
            if let Some(ref output) = self.audio_output {
                output.resume().ok();
            }
            self.last_tick_time = None;
            self.state = PlaybackState::Playing;
        }
        Ok(())
    }

    /// Stop playback and reset to beginning.
    pub fn stop(&mut self) {
        self.stop_internal();
        self.state = PlaybackState::Stopped;
    }

    fn stop_internal(&mut self) {
        self.audio_clock.set_playing(false);
        self.audio_clock.set_position_us(0);
        self.eof.store(false, Ordering::Relaxed);
        self.demux_exhausted = false;

        if let Some(ref output) = self.audio_output {
            output.pause().ok();
        }

        // Flush decoders
        if let Some(ref mut dec) = self.video_decoder {
            dec.flush();
        }
        if let Some(ref mut dec) = self.audio_decoder {
            dec.reset();
        }

        // Clear buffers
        self.video_buffer.clear();
        self.pending_video_packets.clear();
        self.current_frame.store(None);
        if let Some(ref mut ring) = self.audio_ring_buffer {
            ring.clear();
        }
        self.audio_resampler.reset();
        self.audio_spill_buffer.clear();
    }

    /// Seek to a specific time in seconds.
    pub fn seek(&mut self, time_secs: f64) -> Result<()> {
        if self.demuxer.is_none() {
            return Err(anyhow!("No video loaded"));
        }

        let was_playing = self.state == PlaybackState::Playing;

        // Flush decoders
        if let Some(ref mut dec) = self.video_decoder {
            dec.flush();
        }
        if let Some(ref mut dec) = self.audio_decoder {
            dec.reset();
        }

        // Clear buffers
        self.video_buffer.clear();
        self.current_frame.store(None);
        if let Some(ref mut ring) = self.audio_ring_buffer {
            ring.clear();
        }
        self.audio_resampler.reset();
        self.audio_spill_buffer.clear();

        // Seek in demuxer
        if let Some(ref mut demuxer) = self.demuxer {
            demuxer.seek(time_secs)?;
        }

        // Update audio clock
        self.audio_clock
            .set_position_us((time_secs * 1_000_000.0) as i64);
        self.eof.store(false, Ordering::Relaxed);
        self.demux_exhausted = false;

        if was_playing {
            self.audio_clock.set_playing(true);
        }

        Ok(())
    }

    /// Set volume (0.0 - 1.0)
    pub fn set_volume(&mut self, volume: f64) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Set muted state
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// Enable/disable loop
    pub fn set_loop(&mut self, enabled: bool) {
        self.loop_enabled = enabled;
    }

    /// Get current playback state
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// Get the latest decoded frame suitable for the current playback time.
    pub fn current_frame(&self) -> Option<Arc<DecodedFrame>> {
        self.current_frame.load_full()
    }

    /// Get a shared reference to the current frame store (for renderer).
    pub fn current_frame_ref(&self) -> &Arc<ArcSwapOption<DecodedFrame>> {
        &self.current_frame
    }

    /// Get the audio clock position in microseconds.
    pub fn audio_clock_us(&self) -> i64 {
        self.audio_clock.position_us()
    }

    /// Get the total duration in seconds.
    pub fn duration(&self) -> Option<f64> {
        self.duration
    }

    /// Get video dimensions.
    pub fn video_size(&self) -> Option<(u32, u32)> {
        self.video_size
    }

    /// Drive the decode loop forward. Called from the renderer's update() each frame.
    /// This processes packets and fills the video/audio buffers.
    pub fn tick(&mut self) {
        if self.state != PlaybackState::Playing {
            return;
        }

        // If no audio output, advance clock based on real wall time
        if self.audio_output.is_none() && self.audio_clock.is_playing() {
            let now = moyu_pal::time::Instant::now();
            if let Some(last) = self.last_tick_time {
                let delta_us = now.duration_since(last).as_micros() as i64;
                // Cap delta at 100ms to avoid jumps after stalls
                let clamped = delta_us.min(100_000);
                self.audio_clock.advance_us(clamped);
            }
            self.last_tick_time = Some(now);
        }

        let current_time_us = self.audio_clock.position_us();

        // Drain any video frames that arrived asynchronously since the last tick.
        // This is essential for the web backend where WebCodecs decodes via callbacks
        // that fire between ticks, so frames are never available immediately after
        // send_packet().
        self.drain_pending_video_frames();

        // First, try to flush the spill buffer to the ring buffer.
        if let Some(ring) = self.audio_ring_buffer.as_mut() {
            if !self.audio_spill_buffer.is_empty() {
                let written = ring.write(&self.audio_spill_buffer);
                if written > 0 {
                    self.audio_spill_buffer.drain(0..written);
                }
            }
        }

        // Decode more packets if video buffer is low and audio ring buffer has room.
        // When video is absent or failing (video buffer stays empty), the audio ring
        // buffer fill level acts as the sole backpressure signal to prevent decoding
        // audio faster than it can be consumed, which would cause ring buffer overflow
        // and audible skipping.
        let queued_audio_samples = self
            .audio_ring_buffer
            .as_ref()
            .map(|rb| rb.filled_samples())
            .unwrap_or(0)
            + self.audio_spill_buffer.len();
        let audio_ring_full = self
            .audio_ring_buffer
            .as_ref()
            .map(|rb| queued_audio_samples > rb.capacity() * 3 / 4)
            .unwrap_or(false);
        let audio_needs_decode = self
            .audio_ring_buffer
            .as_ref()
            .map(|rb| queued_audio_samples < rb.capacity() / AUDIO_DECODE_LOW_WATERMARK_DIVISOR)
            .unwrap_or(false);
        let video_needs_decode = self.video_buffer.len() < MAX_VIDEO_BUFFER;
        // Also don't decode if we have a lot in the spill buffer
        let allow_audio_decode = self.audio_spill_buffer.len() < AUDIO_SPILL_BUFFER_LIMIT && !audio_ring_full;

        if !audio_needs_decode && video_needs_decode && !self.pending_video_packets.is_empty() {
            self.decode_pending_video_packets();
        }

        let need_more = (video_needs_decode || audio_needs_decode)
            && allow_audio_decode
            && !self.eof.load(Ordering::Relaxed)
            && !self.demux_exhausted;

        if need_more {
            self.decode_packets(video_needs_decode, audio_needs_decode);
        }

        if self.demux_exhausted
            && self.pending_video_packets.is_empty()
            && !self.eof.load(Ordering::Relaxed)
        {
            self.eof.store(true, Ordering::Relaxed);
            self.drain_video_decoder();
        }

        // Select the best frame for the current time
        self.select_frame(current_time_us);

        // Check if we've reached the end
        if self.eof.load(Ordering::Relaxed) && self.video_buffer.is_empty() {
            if !self.is_video_decoder_drained() {
                return;
            }

            if self.loop_enabled {
                // Re-seek to beginning for loop
                if let Err(e) = self.seek(0.0) {
                    log::error!("Failed to loop video: {}", e);
                    self.state = PlaybackState::Error;
                    return;
                }
                self.state = PlaybackState::Playing;
                self.audio_clock.set_playing(true);
            } else {
                self.state = PlaybackState::Ended;
                self.audio_clock.set_playing(false);
            }
        }
    }

    /// Decode a batch of packets from the demuxer.
    fn decode_packets(&mut self, video_needs_decode: bool, audio_needs_decode: bool) {
        // Process up to N packets per tick to avoid blocking the render thread.
        // We collect packets first, then process them to avoid borrow conflicts.
        let mut packets = Vec::new();
        let mut hit_eof = false;
        let max_video_buffer = if audio_needs_decode {
            MAX_VIDEO_BUFFER_WHEN_AUDIO_HUNGRY
        } else {
            MAX_VIDEO_BUFFER
        };
        let max_packet_batch = if audio_needs_decode {
            MAX_PACKET_BATCH_WHEN_AUDIO_HUNGRY
        } else {
            MAX_PACKET_BATCH_NORMAL
        };
        let mut buffered_video_packets = 0usize;

        if let Some(demuxer) = self.demuxer.as_mut() {
            for _ in 0..max_packet_batch {
                if self.video_buffer.len() + buffered_video_packets >= max_video_buffer {
                    break;
                }

                match demuxer.next_packet() {
                    Ok(Some((kind, packet))) => {
                        if kind == TrackKind::Video {
                            buffered_video_packets += 1;
                        }
                        packets.push((kind, packet));
                    }
                    Ok(None) => {
                        hit_eof = true;
                        break;
                    }
                    Err(e) => {
                        log::error!("Demux error: {}", e);
                        hit_eof = true;
                        break;
                    }
                }
            }
        }

        // Process audio first so the output callback can refill sooner when browsers
        // schedule larger or more jittery audio pulls.
        for (kind, packet) in packets.iter() {
            if *kind == TrackKind::Audio {
                self.decode_audio_packet(packet);
            }
        }

        for (kind, packet) in packets {
            match kind {
                TrackKind::Video => {
                    if audio_needs_decode {
                        self.pending_video_packets.push_back(packet);
                    } else if video_needs_decode {
                        self.decode_video_packet(&packet)
                    }
                }
                TrackKind::Audio => {}
            }
        }

        if hit_eof {
            self.demux_exhausted = true;
            if self.pending_video_packets.is_empty() {
                self.eof.store(true, Ordering::Relaxed);
                self.drain_video_decoder();
            }
        }
    }

    fn decode_pending_video_packets(&mut self) {
        while self.video_buffer.len() < MAX_VIDEO_BUFFER {
            let packet = match self.pending_video_packets.pop_front() {
                Some(packet) => packet,
                None => break,
            };
            self.decode_video_packet(&packet);
        }
    }

    fn decode_video_packet(&mut self, packet: &symphonia::core::formats::Packet) {
        // Compute pts_us before borrowing decoder
        let video_time_base = self.video_time_base;
        let pts_us = Self::ts_to_us(packet.ts(), video_time_base);

        let decoder = match self.video_decoder.as_mut() {
            Some(d) => d,
            None => return,
        };

        if let Err(e) = decoder.send_packet(&packet.data, pts_us) {
            log::warn!("Failed to send video packet: {}", e);
            return;
        }

        // Drain decoded frames
        loop {
            match decoder.receive_frame() {
                Ok((DecodeStatus::Ok, Some(frame))) => {
                    if self.video_size.is_none() {
                        self.video_size = Some((frame.width, frame.height));
                    }
                    self.video_buffer.push_back(Arc::new(frame));
                }
                Ok((DecodeStatus::Again, _)) => break,
                Ok((DecodeStatus::Eof, _)) => break,
                Err(e) => {
                    log::warn!("Video decode error: {}", e);
                    break;
                }
                _ => break,
            }
        }
    }

    /// Drain all currently available decoded video frames into the buffer.
    /// On native (FFmpeg) this is mostly a no-op outside of decode_video_packet,
    /// but on web (WebCodecs) frames arrive asynchronously via callbacks and
    /// must be collected each tick.
    fn drain_pending_video_frames(&mut self) {
        let decoder = match self.video_decoder.as_mut() {
            Some(d) => d,
            None => return,
        };

        loop {
            match decoder.receive_frame() {
                Ok((DecodeStatus::Ok, Some(frame))) => {
                    if self.video_size.is_none() {
                        self.video_size = Some((frame.width, frame.height));
                    }
                    self.video_buffer.push_back(Arc::new(frame));
                }
                _ => break,
            }
        }
    }

    fn drain_video_decoder(&mut self) {
        let decoder = match self.video_decoder.as_mut() {
            Some(d) => d,
            None => return,
        };

        // Send empty packet to signal flush
        let _ = decoder.send_packet(&[], 0);

        loop {
            match decoder.receive_frame() {
                Ok((DecodeStatus::Ok, Some(frame))) => {
                    self.video_buffer.push_back(Arc::new(frame));
                }
                _ => break,
            }
        }
    }

    /// Returns true only when the decoder has no more pending output.
    /// WebCodecs flush is asynchronous, so EOF must be confirmed explicitly.
    fn is_video_decoder_drained(&mut self) -> bool {
        let decoder = match self.video_decoder.as_mut() {
            Some(d) => d,
            None => return true,
        };

        loop {
            match decoder.receive_frame() {
                Ok((DecodeStatus::Ok, Some(frame))) => {
                    if self.video_size.is_none() {
                        self.video_size = Some((frame.width, frame.height));
                    }
                    self.video_buffer.push_back(Arc::new(frame));
                    return false;
                }
                Ok((DecodeStatus::Again, _)) => return false,
                Ok((DecodeStatus::Eof, _)) => return true,
                Err(e) => {
                    log::warn!("Video decoder drain check failed: {}", e);
                    return true;
                }
                _ => return false,
            }
        }
    }

    fn decode_audio_packet(&mut self, packet: &symphonia::core::formats::Packet) {
        let (decoder, ring_buffer) =
            match (self.audio_decoder.as_mut(), self.audio_ring_buffer.as_mut()) {
                (Some(d), Some(r)) => (d, r),
                _ => return,
            };

        let dst_rate = self.device_sample_rate;
        let dst_ch = self.device_channels as usize;

        match decoder.decode(packet) {
            Ok(buffer_ref) => {
                // Read actual format from the decoded buffer (more reliable than codec params)
                let src_rate = buffer_ref.spec().rate;
                let src_ch = buffer_ref.spec().channels.count();

                let mut samples = audio_buffer_to_interleaved_f32(&buffer_ref);

                // Channel adaptation: upmix or downmix as needed
                if src_ch != dst_ch {
                    samples = adapt_channels(&samples, src_ch, dst_ch);
                }

                // Sample rate adaptation (stateful linear interpolation)
                let resampled = self
                    .audio_resampler
                    .process(&samples, src_rate, dst_rate, dst_ch);

                let volume = if self.muted { 0.0 } else { self.volume as f32 };

                let scaled: Vec<f32> = if volume != 1.0 {
                    resampled.into_iter().map(|s| s * volume).collect()
                } else {
                    resampled
                };

                if !scaled.is_empty() {
                    let written = ring_buffer.write(&scaled);
                    if written < scaled.len() {
                        self.audio_spill_buffer
                            .extend_from_slice(&scaled[written..]);
                    }
                }
            }
            Err(e) => {
                log::warn!("Audio decode error: {}", e);
            }
        }
    }

    /// Select the frame with PTS closest to but not exceeding the current audio clock.
    fn select_frame(&mut self, current_time_us: i64) {
        let mut best: Option<Arc<DecodedFrame>> = None;

        // Pop all frames that are at or before the current time
        while let Some(front) = self.video_buffer.front() {
            if front.pts_us <= current_time_us {
                best = self.video_buffer.pop_front();
            } else {
                break;
            }
        }

        if let Some(frame) = best {
            self.current_frame.store(Some(frame));
        }
    }

    /// Convert a packet timestamp to microseconds using the track's time base.
    fn ts_to_us(ts: u64, time_base: Option<(u32, u32)>) -> i64 {
        match time_base {
            Some((numer, denom)) if denom > 0 => {
                (ts as f64 * numer as f64 / denom as f64 * 1_000_000.0) as i64
            }
            _ => ts as i64, // fallback: assume timestamp is in microseconds
        }
    }
}

impl Default for VideoPlayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert an AudioBufferRef to interleaved f32 samples.
fn audio_buffer_to_interleaved_f32(buffer: &AudioBufferRef) -> Vec<f32> {
    match buffer {
        AudioBufferRef::F32(buf) => interleave_buffer(buf),
        AudioBufferRef::S16(buf) => interleave_buffer_convert(buf),
        AudioBufferRef::S32(buf) => interleave_buffer_convert(buf),
        AudioBufferRef::U8(buf) => interleave_buffer_convert(buf),
        AudioBufferRef::U16(buf) => interleave_buffer_convert(buf),
        AudioBufferRef::U24(buf) => interleave_buffer_convert(buf),
        AudioBufferRef::U32(buf) => interleave_buffer_convert(buf),
        AudioBufferRef::S8(buf) => interleave_buffer_convert(buf),
        AudioBufferRef::S24(buf) => interleave_buffer_convert(buf),
        AudioBufferRef::F64(buf) => interleave_buffer_convert(buf),
    }
}

fn interleave_buffer(buf: &symphonia::core::audio::AudioBuffer<f32>) -> Vec<f32> {
    let channels = buf.spec().channels.count();
    let frames = buf.frames();
    let mut output = Vec::with_capacity(frames * channels);

    for frame_idx in 0..frames {
        for ch in 0..channels {
            output.push(buf.chan(ch)[frame_idx]);
        }
    }

    output
}

fn interleave_buffer_convert<S: symphonia::core::sample::Sample>(
    buf: &symphonia::core::audio::AudioBuffer<S>,
) -> Vec<f32>
where
    f32: symphonia::core::conv::FromSample<S>,
{
    let channels = buf.spec().channels.count();
    let frames = buf.frames();
    let mut output = Vec::with_capacity(frames * channels);

    for frame_idx in 0..frames {
        for ch in 0..channels {
            output.push(buf.chan(ch)[frame_idx].into_sample());
        }
    }

    output
}

/// Adapt channel count of interleaved audio data.
/// Handles common cases: mono→stereo (duplicate), stereo→mono (average), etc.
fn adapt_channels(samples: &[f32], src_ch: usize, dst_ch: usize) -> Vec<f32> {
    if src_ch == 0 || dst_ch == 0 {
        return Vec::new();
    }

    let frames = samples.len() / src_ch;
    let mut output = Vec::with_capacity(frames * dst_ch);

    for f in 0..frames {
        let src_start = f * src_ch;
        for dc in 0..dst_ch {
            if dc < src_ch {
                output.push(samples[src_start + dc]);
            } else {
                // Duplicate the last available source channel (e.g. mono→stereo)
                output.push(samples[src_start + (src_ch - 1).min(dc)]);
            }
        }
    }

    output
}

#[derive(Debug, Clone)]
struct AudioResampler {
    raw_buffer: Vec<f32>,
    src_pos: f64,
}

impl AudioResampler {
    fn new() -> Self {
        Self {
            raw_buffer: Vec::new(),
            src_pos: 0.0,
        }
    }

    fn process(
        &mut self,
        new_samples: &[f32],
        src_rate: u32,
        dst_rate: u32,
        channels: usize,
    ) -> Vec<f32> {
        if channels == 0 || src_rate == 0 || dst_rate == 0 {
            return Vec::new();
        }

        if src_rate == dst_rate {
            // Passthrough, no need to buffer.
            return new_samples.to_vec();
        }

        self.raw_buffer.extend_from_slice(new_samples);
        let ratio = src_rate as f64 / dst_rate as f64;
        let available_src_frames = self.raw_buffer.len() / channels;

        if available_src_frames == 0 {
            return Vec::new();
        }

        let mut output = Vec::new();
        // We need at least 2 frames (index and index+1) to interpolate cleanly
        while self.src_pos < (available_src_frames - 1) as f64 {
            let idx = self.src_pos as usize;
            let frac = (self.src_pos - idx as f64) as f32;

            for ch in 0..channels {
                let s0 = self.raw_buffer[idx * channels + ch];
                let s1 = self.raw_buffer[(idx + 1) * channels + ch];
                output.push(s0 + (s1 - s0) * frac);
            }

            self.src_pos += ratio;
        }

        let frames_to_remove = self.src_pos as usize;
        if frames_to_remove > 0 {
            self.raw_buffer.drain(0..frames_to_remove * channels);
            self.src_pos -= frames_to_remove as f64;
        }

        output
    }

    fn reset(&mut self) {
        self.raw_buffer.clear();
        self.src_pos = 0.0;
    }
}
