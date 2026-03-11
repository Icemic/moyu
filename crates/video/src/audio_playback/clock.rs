use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

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
