use anyhow::{Result, anyhow};
use web_sys::{AudioBufferSourceNode, AudioContext, GainNode};

use super::clock::AudioClock;
use super::ring_buffer::AudioRingBuffer;

/// Chunk size in frames to schedule into the Web Audio API at a time.
/// ~2048 frames at 48 kHz ≈ 42 ms per chunk — small enough for low latency
/// while keeping call overhead negligible.
const CHUNK_FRAMES: usize = 2048;

/// How far ahead (in seconds) we schedule audio buffers on the AudioContext
/// timeline.  The browser's audio rendering thread will consume these
/// autonomously — no main-thread callback is involved.
const SCHEDULE_AHEAD_SECS: f64 = 0.15;

/// Web Audio API based audio output.
///
/// Instead of cpal's `ScriptProcessorNode` (deprecated, runs on the main
/// thread, competes with WASM decode work), this uses the standard
/// schedule-ahead pattern with `AudioBufferSourceNode`:
///
/// 1. Each `pump()` call (driven from `tick()`) reads decoded samples from the
///    ring buffer.
/// 2. Samples are written into `AudioBuffer` objects and scheduled at precise
///    times on the `AudioContext.currentTime` timeline via `start(when)`.
/// 3. The browser's **high-priority audio thread** plays them — zero main-thread
///    involvement during playback.
pub struct WebAudioOutput {
    ctx: AudioContext,
    gain: GainNode,
    pub clock: AudioClock,
    pub sample_rate: u32,
    pub channels: u16,
    /// Next absolute time (in AudioContext seconds) at which audio should be
    /// scheduled.  Advances as we push chunks.
    next_schedule_time: f64,
    /// Whether we have started scheduling (first pump after play/resume).
    started: bool,
}

// Safety: WASM is single-threaded; AudioContext / GainNode (JsValue) are only
// accessed from the main thread.  The Node trait requires Send + Sync because
// native platforms use multi-threaded rendering, but on wasm32 this is a no-op.
unsafe impl Send for WebAudioOutput {}
unsafe impl Sync for WebAudioOutput {}

impl WebAudioOutput {
    pub fn new(
        _requested_sample_rate: u32,
        _requested_channels: u16,
        clock: AudioClock,
    ) -> Result<Self> {
        let ctx = AudioContext::new().map_err(|e| anyhow!("AudioContext::new failed: {:?}", e))?;
        let sample_rate = ctx.sample_rate() as u32;
        // Web Audio API always outputs stereo to the destination.
        let channels = ctx.destination().channel_count() as u16;

        let gain = GainNode::new(&ctx).map_err(|e| anyhow!("GainNode::new failed: {:?}", e))?;
        gain.connect_with_audio_node(&ctx.destination())
            .map_err(|e| anyhow!("gain.connect failed: {:?}", e))?;

        log::info!(
            "WebAudioOutput: ctx sample_rate={}Hz channels={}",
            sample_rate,
            channels,
        );

        Ok(Self {
            ctx,
            gain,
            clock,
            sample_rate,
            channels,
            next_schedule_time: 0.0,
            started: false,
        })
    }

    /// Push decoded audio from the ring buffer into the Web Audio graph.
    /// Should be called every tick.
    pub fn pump(&mut self, ring: &mut AudioRingBuffer) {
        if !self.clock.is_playing() {
            return;
        }

        let sr = self.sample_rate as f64;
        let ch = self.channels as usize;
        let current_time = self.ctx.current_time();

        // On the very first pump (or after a resume) align the schedule cursor
        // to "now" so we don't try to schedule into the past.
        if !self.started {
            self.next_schedule_time = current_time;
            self.started = true;
        }

        // If next_schedule_time fell behind current_time (e.g. tab was
        // backgrounded), snap forward.
        if self.next_schedule_time < current_time {
            self.next_schedule_time = current_time;
        }

        let schedule_horizon = current_time + SCHEDULE_AHEAD_SECS;

        // Keep scheduling CHUNK_FRAMES-sized buffers until we've filled the
        // look-ahead window or we run out of data.
        let mut tmp = vec![0.0f32; CHUNK_FRAMES * ch];
        while self.next_schedule_time < schedule_horizon {
            let available = ring.filled_samples();
            let needed = CHUNK_FRAMES * ch;
            if available < needed {
                break; // Not enough data yet — will catch up next tick.
            }

            // Read interleaved samples from the ring buffer.
            let read = ring.read_direct(&mut tmp[..needed]);
            if read < needed {
                break;
            }

            // Advance the clock by what we just scheduled.
            let frames = read / ch;
            let delta_us = (frames as f64 / sr * 1_000_000.0) as i64;
            self.clock.advance_us(delta_us);

            // Create an AudioBuffer and de-interleave into per-channel arrays.
            let audio_buffer = self
                .ctx
                .create_buffer(ch as u32, frames as u32, self.sample_rate as f32)
                .expect("create_buffer failed");

            for c in 0..ch {
                // Gather channel c from interleaved data.
                let channel_data: Vec<f32> = (0..frames).map(|f| tmp[f * ch + c]).collect();
                audio_buffer
                    .copy_to_channel(&channel_data, c as i32)
                    .expect("copy_to_channel failed");
            }

            // Schedule playback.
            let source: AudioBufferSourceNode = self
                .ctx
                .create_buffer_source()
                .expect("create_buffer_source failed");
            source.set_buffer(Some(&audio_buffer));
            source
                .connect_with_audio_node(&self.gain)
                .expect("source.connect failed");
            source
                .start_with_when(self.next_schedule_time)
                .expect("source.start failed");

            self.next_schedule_time += frames as f64 / sr;
        }
    }

    pub fn pause(&self) -> Result<()> {
        self.clock.set_playing(false);
        let _ = self.ctx.suspend();
        Ok(())
    }

    pub fn resume(&self) -> Result<()> {
        self.clock.set_playing(true);
        let _ = self.ctx.resume();
        Ok(())
    }

    pub fn reset_schedule(&mut self) {
        self.started = false;
        self.next_schedule_time = 0.0;
    }
}
