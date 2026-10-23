//! Model loading request.

use crate::types::Span;

/// Loading parameters for one model.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ModelDesc {
    /// Path of the `.skel` or `.json` skeleton, relative to the asset root.
    pub skeleton_path: Span,
    /// Path of the `.atlas` file, relative to the asset root. An empty span
    /// selects the `.atlas` next to the skeleton with the same base name.
    pub atlas_path: Span,
}
