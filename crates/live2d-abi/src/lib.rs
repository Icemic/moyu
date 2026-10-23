//! C ABI contract between the Moyu engine (`crates/live2d`) and the
//! closed-source `moyu_live2d_plugin` binary.
//!
//! Single source of truth for the `repr(C)` types and version constants shared
//! across the plugin boundary. Contains no Live2D SDK content.
//!
//! # Conventions
//!
//! - Every entry point returns an `i32` error code defined in [`error`];
//!   [`error::OK`] is success. After a failure the plugin-side message is read
//!   through the `last_error` entry point as a [`Span`], valid until the next call.
//! - Strings are UTF-8 spans without NUL termination; an empty span means
//!   "absent".
//! - List queries are two-phase: a probe call reports the entry count, then
//!   each entry is read by index. Query results stay valid until the model is
//!   unloaded.
//! - The frame snapshot returned by `get_frame` is owned by the plugin and
//!   stays valid **until the next `update` or unload**; the host must not keep
//!   pointers across frames.
//! - All calls happen on the engine main thread; the plugin needs no internal
//!   locking, and the host must not reenter the plugin from `read_file`.

pub mod command;
pub mod error;
pub mod frame;
pub mod init;
pub mod model;
pub mod query;
pub mod types;
pub mod version;

pub use command::ParameterPair;
pub use frame::{DrawableDesc, FrameDesc, MaskBufferDesc, MaskCommand};
pub use init::{PluginInit, ReadFileFn};
pub use model::ModelDesc;
pub use query::{
    CanvasInfo, EventDesc, ExpressionInfo, HitAreaInfo, MotionInfo, ParameterInfo, PartInfo,
    TextureInfo,
};
pub use types::{Matrix4, Span, Vec2};

