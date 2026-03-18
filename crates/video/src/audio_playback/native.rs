use anyhow::{Result, anyhow};

use super::clock::AudioClock;
use super::ring_buffer::AudioRingBufferReader;

// ── Audio output (cpal, native only) ──────────────────────────────────────────
//
// On web (wasm32), we bypass cpal entirely and use the Web Audio API directly
// via `WebAudioOutput` in web_audio.rs.  cpal's web backend uses the deprecated
// ScriptProcessorNode which runs audio callbacks on the main thread, competing
// with our WASM decode work and causing persistent stutter in Chrome.

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

        let actual_sample_rate = default_config.sample_rate();
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
