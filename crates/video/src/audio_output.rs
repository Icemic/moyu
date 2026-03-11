use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use anyhow::{Result, anyhow};

/// Audio clock for A/V synchronization.
///
/// Tracks how much audio has been consumed by the output device callback.
/// The design follows the same principle as Kira: keep it simple, let the
/// OS / browser handle scheduling, and don't try to outsmart the platform.
#[derive(Debug, Clone)]
pub struct AudioClock {
    /// Cumulative audio time consumed by the output callback, in microseconds.
    position_us: Arc<AtomicI64>,
    /// Whether audio is actively playing
    playing: Arc<AtomicBool>,
}

impl AudioClock {
    pub fn new() -> Self {
        Self {
            position_us: Arc::new(AtomicI64::new(0)),
            playing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get current playback position in microseconds (how much audio the device consumed).
    pub fn position_us(&self) -> i64 {
        self.position_us.load(Ordering::Relaxed)
    }

    /// Set the playback position in microseconds.
    pub fn set_position_us(&self, us: i64) {
        self.position_us.store(us, Ordering::Relaxed);
    }

    /// Advance the clock by the given number of microseconds.
    pub fn advance_us(&self, delta_us: i64) {
        self.position_us.fetch_add(delta_us, Ordering::Relaxed);
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    pub fn set_playing(&self, playing: bool) {
        self.playing.store(playing, Ordering::Relaxed);
    }
}

/// Ring buffer for audio samples (interleaved f32).
/// Thread-safe SPSC (single producer, single consumer).
pub struct AudioRingBuffer {
    buffer: Vec<f32>,
    read_pos: Arc<std::sync::atomic::AtomicUsize>,
    write_pos: Arc<std::sync::atomic::AtomicUsize>,
    capacity: usize,
}

impl AudioRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            read_pos: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            write_pos: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            capacity,
        }
    }

    pub fn write(&mut self, data: &[f32]) -> usize {
        let read = self.read_pos.load(Ordering::Acquire);
        let write = self.write_pos.load(Ordering::Relaxed);
        let available = if write >= read {
            self.capacity - (write - read) - 1
        } else {
            read - write - 1
        };
        let to_write = data.len().min(available);

        for i in 0..to_write {
            let idx = (write + i) % self.capacity;
            self.buffer[idx] = data[i];
        }

        self.write_pos
            .store((write + to_write) % self.capacity, Ordering::Release);
        to_write
    }

    /// Returns the number of samples currently held in the buffer.
    pub fn filled_samples(&self) -> usize {
        let read = self.read_pos.load(Ordering::Relaxed);
        let write = self.write_pos.load(Ordering::Acquire);
        if write >= read {
            write - read
        } else {
            self.capacity - read + write
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn clear(&mut self) {
        self.read_pos.store(0, Ordering::Release);
        self.write_pos.store(0, Ordering::Release);
    }

    /// Create a reader handle that can be sent to the audio callback thread.
    pub fn reader(&self) -> AudioRingBufferReader {
        AudioRingBufferReader {
            buffer_ptr: self.buffer.as_ptr(),
            capacity: self.capacity,
            read_pos: self.read_pos.clone(),
            write_pos: self.write_pos.clone(),
        }
    }
}

/// Read-only handle to the ring buffer. Safe to send to audio thread.
pub struct AudioRingBufferReader {
    buffer_ptr: *const f32,
    capacity: usize,
    read_pos: Arc<std::sync::atomic::AtomicUsize>,
    write_pos: Arc<std::sync::atomic::AtomicUsize>,
}

// Safety: The buffer pointer points to valid memory that outlives the reader
// (the writer keeps the Vec alive), and read/write are atomic.
unsafe impl Send for AudioRingBufferReader {}

impl AudioRingBufferReader {
    pub fn available_samples(&self) -> usize {
        let read = self.read_pos.load(Ordering::Relaxed);
        let write = self.write_pos.load(Ordering::Acquire);
        if write >= read {
            write - read
        } else {
            self.capacity - read + write
        }
    }

    pub fn read(&self, output: &mut [f32]) -> usize {
        let read = self.read_pos.load(Ordering::Relaxed);
        let available = self.available_samples();
        let to_read = output.len().min(available);

        for i in 0..to_read {
            let idx = (read + i) % self.capacity;
            output[i] = unsafe { *self.buffer_ptr.add(idx) };
        }

        self.read_pos
            .store((read + to_read) % self.capacity, Ordering::Release);
        to_read
    }
}

// ── Audio output (cpal, works on both native and WASM via WebAudio) ───────────

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct AudioOutput {
    stream: cpal::Stream,
    pub clock: AudioClock,
    /// Actual device sample rate (may differ from source)
    pub sample_rate: u32,
    /// Actual device channel count (may differ from source)
    pub channels: u16,
}

// Safety: cpal::Stream is not Send on some platforms, but we only
// control it from the main thread. The playback callback runs separately.
unsafe impl Send for AudioOutput {}

impl AudioOutput {
    /// Create a new audio output stream.
    /// `requested_sample_rate` and `requested_channels` are hints from the source;
    /// the actual output config is negotiated with the device.
    pub fn new(
        requested_sample_rate: u32,
        requested_channels: u16,
        reader: AudioRingBufferReader,
        clock: AudioClock,
    ) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("No audio output device available"))?;

        // Use device's preferred config, same approach as Kira.
        let default_config = device
            .default_output_config()
            .map_err(|e| anyhow!("Failed to get default output config: {}", e))?;

        let actual_sample_rate = default_config.sample_rate().0;
        let actual_channels = default_config.channels();
        let config = default_config.config();

        log::info!(
            "Audio output: requested {}Hz {}ch, device {}Hz {}ch",
            requested_sample_rate,
            requested_channels,
            actual_sample_rate,
            actual_channels,
        );

        let clock_clone = clock.clone();
        let us_per_sample = 1_000_000.0 / actual_sample_rate as f64;
        let ch = actual_channels as usize;

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    if !clock_clone.is_playing() {
                        data.fill(0.0);
                        return;
                    }

                    let read = reader.read(data);

                    // Fill remaining with silence (underrun — no drama, just silence).
                    if read < data.len() {
                        data[read..].fill(0.0);
                    }

                    // Advance clock only for samples actually consumed from the buffer.
                    let frames = read / ch;
                    let delta_us = (frames as f64 * us_per_sample) as i64;
                    clock_clone.advance_us(delta_us);
                },
                |err| {
                    log::error!("Audio output stream error: {}", err);
                },
                None,
            )
            .map_err(|e| anyhow!("Failed to build audio output stream: {}", e))?;

        stream
            .play()
            .map_err(|e| anyhow!("Failed to play audio stream: {}", e))?;

        Ok(Self {
            stream,
            clock,
            sample_rate: actual_sample_rate,
            channels: actual_channels,
        })
    }

    pub fn pause(&self) -> Result<()> {
        self.clock.set_playing(false);
        self.stream
            .pause()
            .map_err(|e| anyhow!("Failed to pause audio: {}", e))
    }

    pub fn resume(&self) -> Result<()> {
        self.clock.set_playing(true);
        self.stream
            .play()
            .map_err(|e| anyhow!("Failed to resume audio: {}", e))
    }
}
