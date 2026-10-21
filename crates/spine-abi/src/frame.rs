//! Per-frame rendering snapshot produced by `get_frame`.
//!
//! The snapshot is a transient draw stream: batch count, order and ownership
//! may change every frame. The host copies what it needs before calling back
//! into the plugin and submits batches in the order given.

use crate::types::Vec2;

/// Batches blend normally: source premultiplied color over the destination.
pub const BLEND_NORMAL: i32 = 0;

/// Batches add their color to the destination.
pub const BLEND_ADDITIVE: i32 = 1;

/// Batches multiply the destination by their color.
pub const BLEND_MULTIPLY: i32 = 2;

/// Batches invert the destination by their color.
pub const BLEND_SCREEN: i32 = 3;

/// Root of a frame snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FrameDesc {
    /// Batches in draw order.
    pub batches: *const BatchDesc,
    /// Number of batches.
    pub batch_count: usize,
    /// Total vertices across all batches, for host-side limit checks.
    pub vertex_count_total: usize,
    /// Total triangle indices across all batches.
    pub index_count_total: usize,
}

/// One batch: triangles sharing a texture page, blend mode and vertex tint.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BatchDesc {
    /// Index into the model's page list. Out of range is a contract violation.
    pub texture_page_index: i32,
    /// Effective blend mode, one of `BLEND_*`.
    pub blend_mode: i32,
    /// Reserved; zero in this version.
    pub flags: u32,
    /// Number of vertices.
    pub vertex_count: usize,
    /// Number of triangle indices.
    pub index_count: usize,
    /// Vertex positions in world space, y up.
    pub positions: *const Vec2,
    /// Normalized UVs, top-left origin, y down.
    pub uvs: *const Vec2,
    /// Light color per vertex, `u32 = b | g << 8 | r << 16 | a << 24`, RGB
    /// premultiplied by its own alpha.
    pub colors: *const u32,
    /// Dark color per vertex in the same packing; RGB premultiplied by the
    /// light alpha, and the alpha byte itself is always 255.
    pub dark_colors: *const u32,
    /// Triangle indices, `u16` each.
    pub indices: *const u16,
}

/// Packs an RGBA8 color the way the runtime lays it out in memory on
/// little-endian targets.
pub const fn pack_color(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (b as u32) | ((g as u32) << 8) | ((r as u32) << 16) | ((a as u32) << 24)
}
