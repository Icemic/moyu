//! Atlas page description reported once per model.
//!
//! A page may hold one or more atlas regions; a single `.atlas` file usually
//! declares several pages. The host loads each page texture with the sampling
//! state given here and normalizes its alpha to premultiplied once, at upload.

use crate::types::Span;

/// Page pixels are stored as straight (non-premultiplied) alpha.
pub const ALPHA_STRAIGHT: i32 = 0;

/// Page pixels already carry premultiplied alpha.
pub const ALPHA_PREMULTIPLIED: i32 = 1;

/// Nearest-neighbor sampling, no mipmaps.
pub const PAGE_FILTER_NEAREST: i32 = 0;

/// Linear sampling, no mipmaps.
pub const PAGE_FILTER_LINEAR: i32 = 1;

/// Nearest sampling between mip levels.
pub const PAGE_FILTER_MIPMAP_NEAREST: i32 = 2;

/// Linear sampling between mip levels.
pub const PAGE_FILTER_MIPMAP_LINEAR: i32 = 3;

/// Clamp to edge.
pub const PAGE_WRAP_CLAMP: i32 = 0;

/// Repeat.
pub const PAGE_WRAP_REPEAT: i32 = 1;

/// Mirrored repeat.
pub const PAGE_WRAP_MIRROR: i32 = 2;

/// One atlas page.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PageDesc {
    /// Texture path relative to the asset root (`assets/`).
    pub path: Span,
    /// Page width in pixels; must be positive, since UVs are computed from it.
    pub width: u32,
    /// Page height in pixels; must be positive.
    pub height: u32,
    /// Input alpha encoding, `ALPHA_*`.
    pub alpha: i32,
    /// Minification filter, `PAGE_FILTER_*`.
    pub min_filter: i32,
    /// Magnification filter, `PAGE_FILTER_*`.
    pub mag_filter: i32,
    /// Horizontal wrap, `PAGE_WRAP_*`.
    pub wrap_u: i32,
    /// Vertical wrap, `PAGE_WRAP_*`.
    pub wrap_v: i32,
}
