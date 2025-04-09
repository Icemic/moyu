use moyu_pal::time::Instant;
use std::cell::Cell;

/// A high-performance, lock-free FPS meter
///
/// Used to calculate and track the application's frame rate
pub struct FpsMeter {
    // Timestamp of the last counter reset
    last_reset: Cell<Instant>,
    // Frame count in the current interval
    frame_count: Cell<u32>,
    // Calculated FPS value
    current_fps: Cell<f32>,
    // Update interval (in seconds)
    update_interval: f32,
}

impl FpsMeter {
    /// Creates a new FPS meter
    ///
    /// # Parameters
    /// * `update_interval` - Time interval for updating FPS (in seconds). Defaults to 1.0 second.
    pub fn new(update_interval: Option<f32>) -> Self {
        Self {
            last_reset: Cell::new(Instant::now()),
            frame_count: Cell::new(0),
            current_fps: Cell::new(0.0),
            update_interval: update_interval.unwrap_or(1.0),
        }
    }

    /// Records a frame and updates FPS when necessary
    ///
    /// Call this method to notify the FPS meter that a frame has been rendered
    pub fn tick(&self) -> bool {
        // Increment frame count
        let new_count = self.frame_count.get() + 1;
        self.frame_count.set(new_count);

        let now = Instant::now();
        let duration = now.duration_since(self.last_reset.get()).as_secs_f32();

        // Check if FPS needs to be updated
        if duration >= self.update_interval {
            // Calculate FPS
            let fps = new_count as f32 / duration;
            self.current_fps.set(fps);

            // Reset counters
            self.frame_count.set(0);
            self.last_reset.set(now);

            return true; // FPS has been updated
        }

        false // FPS not updated
    }

    /// Gets the current calculated FPS value
    pub fn get_fps(&self) -> f32 {
        self.current_fps.get()
    }

    /// Formats the FPS as a human-readable string
    pub fn format_fps(&self) -> String {
        format!("fps: {:.1}", self.get_fps())
    }
}

impl Default for FpsMeter {
    fn default() -> Self {
        Self::new(None)
    }
}
