use std::sync::Arc;
use std::sync::atomic::Ordering;

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

    /// Read samples directly from the ring buffer (used by web audio pump
    /// on the same thread that writes, so no separate reader needed).
    #[allow(dead_code)]
    pub fn read_direct(&mut self, output: &mut [f32]) -> usize {
        let read = self.read_pos.load(Ordering::Relaxed);
        let write = self.write_pos.load(Ordering::Acquire);
        let available = if write >= read {
            write - read
        } else {
            self.capacity - read + write
        };
        let to_read = output.len().min(available);

        for i in 0..to_read {
            let idx = (read + i) % self.capacity;
            output[i] = self.buffer[idx];
        }

        self.read_pos
            .store((read + to_read) % self.capacity, Ordering::Release);
        to_read
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
