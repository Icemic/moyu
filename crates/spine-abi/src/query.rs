//! Model metadata available after load and valid until unload, plus the
//! pull-based event queue.
//!
//! List entries are read two-phase: a probe call reports the entry count, then
//! each entry is fetched by index. Entry strings are UTF-8 spans owned by the
//! plugin and stay valid until the model is unloaded.

use crate::types::{Bounds, Span};

/// Model geometry and version information.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ModelInfo {
    /// Skeleton data version string as written by the Spine Editor export.
    /// Diagnostic only: it never selects a runtime.
    pub data_version: Span,
    /// Reference bounds measured once at load in the setup pose, in world
    /// units, with the initial skin applied. Zero-sized when the model has no
    /// measurable geometry.
    pub reference_bounds: Bounds,
}

/// Plugin build information, for diagnostics.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PluginInfo {
    /// Runtime version string the plugin was compiled against.
    pub runtime_version: Span,
    /// Runtime source revision the plugin was built from.
    pub runtime_revision: Span,
    /// Contract version the plugin implements, `major << 16 | minor`.
    pub abi_version: u32,
}

/// One animation.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AnimationInfo {
    /// Animation name.
    pub name: Span,
    /// Duration in seconds.
    pub duration: f32,
}

/// One skin.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SkinInfo {
    /// Skin name; the default skin has an empty name.
    pub name: Span,
}

/// One bounding box attachment, used by hit testing and debugging.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BoundingBoxInfo {
    /// Bounding box name.
    pub name: Span,
}
