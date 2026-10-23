//! Shared value types.

use core::ptr;
use core::str;

/// Two-component vector.
pub type Vec2 = [f32; 2];

/// Row-major 4x4 matrix.
pub type Matrix4 = [[f32; 4]; 4];

/// UTF-8 string view: pointer plus byte length, without NUL termination.
///
/// An empty span (null pointer or zero length) means "absent". The backing
/// memory is owned by the producing side and stays valid according to the entry
/// point that returned it.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub ptr: *const u8,
    pub len: usize,
}

impl Span {
    /// Empty span, used for absent strings.
    pub const EMPTY: Span = Span {
        ptr: ptr::null(),
        len: 0,
    };

    /// Creates a span borrowing `bytes`; the caller keeps the memory alive.
    pub const fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            ptr: bytes.as_ptr(),
            len: bytes.len(),
        }
    }

    /// Views the span as UTF-8; `None` when empty or not valid UTF-8.
    ///
    /// # Safety
    ///
    /// `ptr` must point to `len` readable bytes.
    pub unsafe fn as_str(&self) -> Option<&str> {
        if self.ptr.is_null() || self.len == 0 {
            return None;
        }
        unsafe { str::from_utf8(core::slice::from_raw_parts(self.ptr, self.len)).ok() }
    }
}
