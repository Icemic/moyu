//! C ABI contract between the Moyu engine (`crates/spine`) and the
//! `moyu_spine_plugin` binary.
//!
//! Single source of truth for the `repr(C)` types and version constants shared
//! across the plugin boundary. Contains no Spine runtime content.
//!
//! # Conventions
//!
//! - Every entry point returns an `i32` error code defined in [`error`];
//!   [`error::OK`] is success. After a failure the plugin-side message is read
//!   through the `last_error` entry point as a [`Span`], valid until the next
//!   call.
//! - Strings are UTF-8 spans without NUL termination; an empty span means
//!   "absent".
//! - List queries are two-phase: a probe call reports the entry count, then each
//!   entry is read by index. Query results stay valid until the model is
//!   unloaded.
//! - The frame snapshot returned by `get_frame` is a transient draw stream owned
//!   by the plugin and stays valid **until the next call of any entry point**,
//!   not merely until the next `update`. The host must copy everything it needs
//!   before calling back into the plugin.
//! - Batches are submitted in the order the plugin produced them. The host must
//!   not regroup them by texture or blend mode.
//! - All calls happen on the engine main thread; the plugin needs no internal
//!   locking, and the host must not reenter the plugin from `read_file`.

pub mod error;
pub mod event;
pub mod frame;
pub mod init;
pub mod model;
pub mod page;
pub mod query;
pub mod types;
pub mod version;

pub use event::EventDesc;
pub use frame::{BLEND_ADDITIVE, BLEND_MULTIPLY, BLEND_NORMAL, BLEND_SCREEN, BatchDesc, FrameDesc, pack_color};
pub use init::{PluginInit, ReadFileFn};
pub use model::ModelDesc;
pub use page::{ALPHA_PREMULTIPLIED, ALPHA_STRAIGHT, PageDesc};
pub use query::{
    AnimationInfo, BoundingBoxInfo, ModelInfo, PluginInfo, SkinInfo,
};
pub use types::{Bounds, Span, Vec2};
