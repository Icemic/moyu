//! Logger wrapper.
//!
//! The engine installs the platform logger through [`create_logger`], so that every
//! record the backend accepts also reaches the log buffer. Records are only kept once
//! the bridge has started, which keeps a plain engine free of buffering.

use crate::logs;

/// Wrap a platform logger so that every record it accepts is also kept for `engine:logs`.
pub fn create_logger(backend: Box<dyn log::Log>) -> Box<dyn log::Log> {
    Box::new(Recorder { backend })
}

/// Forwards each record to the platform backend and into the log buffer.
struct Recorder {
    backend: Box<dyn log::Log>,
}

impl log::Log for Recorder {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.backend.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if !self.backend.enabled(record.metadata()) {
            return;
        }

        self.backend.log(record);
        logs::record(record);
    }

    fn flush(&self) {
        self.backend.flush();
    }
}
