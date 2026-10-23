//! Model loading request.

use crate::types::Span;

/// Loading parameters for one model.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ModelDesc {
    /// Path of the `.model3.json`, relative to the asset root. Paths of every
    /// referenced asset are resolved against the model directory by the plugin.
    pub model_path: Span,
}
