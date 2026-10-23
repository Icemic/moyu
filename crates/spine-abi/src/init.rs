//! Plugin initialization payload.

use core::ffi::c_void;

use crate::types::Span;

/// Host file access callback.
///
/// Two-phase: with a null `buffer` the callback reports the number of bytes
/// available from `offset` in `inout_len`; otherwise it copies up to
/// `*inout_len` bytes into `buffer` and reports the number of bytes copied.
/// `path` is always relative to the asset root (`assets/`), and the bytes are
/// valid until the callback returns. Runs on the engine main thread, which must
/// not reenter the plugin. Returns `OK` on success.
pub type ReadFileFn = unsafe extern "C" fn(
    user_data: *mut c_void,
    path: Span,
    offset: u64,
    buffer: *mut u8,
    inout_len: *mut usize,
) -> i32;

/// Initialization payload passed to the plugin `create` entry point.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PluginInit {
    /// File access callback; the host always provides it.
    pub read_file: Option<ReadFileFn>,
    /// Opaque value passed back to `read_file`.
    pub user_data: *mut c_void,
}
