//! Per-frame rendering snapshot produced by `update`.
//!
//! The snapshot and everything it points to are owned by the plugin and stay
//! valid until the next `update` or until the model is unloaded; the host must
//! not keep pointers across frames.

use crate::types::{Matrix4, Vec2};

/// Drawable is visible.
pub const FLAG_VISIBLE: u32 = 1 << 0;

/// Drawable is double-sided; single-sided drawables cull back faces.
pub const FLAG_DOUBLE_SIDED: u32 = 1 << 1;

/// Inverted mask: the mask coefficient is the sampled channel value instead of
/// its complement.
pub const FLAG_MASK_INVERTED: u32 = 1 << 2;

/// Blend mode lies outside the supported set (normal/blend/add); the host
/// reports it once.
pub const FLAG_BLEND_UNSUPPORTED: u32 = 1 << 3;

/// Drawable blends normally: source premultiplied color over the destination.
pub const BLEND_NORMAL: i32 = 0;

/// Drawable adds its color to the destination.
pub const BLEND_ADD: i32 = 1;

/// Drawable multiplies the destination by its color.
pub const BLEND_MULTIPLY: i32 = 2;

/// Root of a frame snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FrameDesc {
    /// Canvas dimensions in canvas pixels.
    pub canvas_size: Vec2,
    /// Model origin on the canvas, in canvas pixels.
    pub canvas_origin: Vec2,
    /// Scaling from canvas pixels to model units.
    pub pixels_per_unit: f32,
    /// Drawables in draw order.
    pub drawables: *const DrawableDesc,
    /// Number of drawables.
    pub drawable_count: usize,
    /// Mask render targets the host must provide for this frame; empty when the
    /// model needs no masks.
    pub mask_buffers: *const MaskBufferDesc,
    /// Number of mask buffers.
    pub mask_buffer_count: usize,
    /// Mask draw commands in execution order.
    pub mask_commands: *const MaskCommand,
    /// Number of mask commands.
    pub mask_command_count: usize,
}

/// Per-drawable snapshot. Geometry pointers reference plugin-owned memory.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrawableDesc {
    /// `FLAG_*` bitfield.
    pub flags: u32,
    /// Blend mode the drawable must be drawn with, one of `BLEND_*`.
    ///
    /// The plugin has already applied the fallback: a drawable whose native
    /// mode is unsupported carries `BLEND_NORMAL` together with
    /// `FLAG_BLEND_UNSUPPORTED`, so the host never interprets a native value.
    pub blend_mode: i32,
    /// The mode the model declared, opaque to the host.
    ///
    /// Only for diagnostics: it is what `blendModeUnsupported` reports back to
    /// the project, and the host must not draw with it.
    pub source_blend_mode: i32,
    /// Index into the texture list.
    pub texture_index: i32,
    /// Drawable opacity with the model opacity folded in; the host applies it once.
    pub opacity: f32,
    /// Per-channel multiply color (RGB).
    pub multiply: [f32; 3],
    /// Per-channel screen color (RGB).
    pub screen: [f32; 3],
    /// Number of vertices.
    pub vertex_count: usize,
    /// Number of triangle indices.
    pub index_count: usize,
    /// Vertex positions in canvas pixel space (top-left origin, y down).
    pub vertices: *const Vec2,
    /// Normalized UVs (top-left origin, y down; flipping already applied).
    pub uv: *const Vec2,
    /// Triangle indices, `u16` each.
    pub indices: *const u16,
    /// Mask buffer this drawable samples; -1 when unmasked.
    pub mask_buffer_index: i32,
    /// Mask channel inside the buffer, 0-3.
    pub mask_channel: u32,
    /// Transform from snapshot vertex space to mask sampling coordinates
    /// (UV, 0..1; v = 0 is the first row of the mask buffer).
    pub clip_matrix: Matrix4,
}

/// A mask render target the host provides as an RGBA8 texture. Every buffer
/// has the same size; count and size are decided by the plugin.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MaskBufferDesc {
    pub width: u32,
    pub height: u32,
}

/// One mask draw command, executed in order by the host.
///
/// Every mask buffer is cleared to 1.0 once at the start of the frame, before
/// any of its commands run; the plugin groups the clearing that way, so a
/// command carries no per-command clear flag.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MaskCommand {
    /// Target mask buffer.
    pub buffer_index: u32,
    /// Channel written by this command, 0-3.
    pub channel: u32,
    /// Index into `FrameDesc::drawables` providing geometry and texture.
    pub drawable_index: usize,
    /// Transform from snapshot vertex space to mask clip space, used for
    /// rasterization; y = +1 falls on the first row of the mask buffer, so the
    /// same point maps to the same row when sampled through `clip_matrix`.
    pub matrix: Matrix4,
    /// Rectangle `[x_min, y_min, x_max, y_max]` in the same clip space as
    /// `matrix`; fragments outside it do not write to the mask.
    pub clip_rect: [f32; 4],
}
