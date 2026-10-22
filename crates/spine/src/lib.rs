//! Spine integration for the Moyu engine: the `spine` node and its wgpu
//! renderer.
//!
//! All Spine semantics and rendering data come from the `moyu_spine_plugin`
//! binary, loaded at runtime via `libloading`. The Spine runtime is compiled
//! into that plugin, so it never enters this repository.
//!
//! # Threading
//!
//! The plugin is pinned to one thread: the script thread, where the engine polls
//! plugins and ticks the script runtime. The node drives it from there (props,
//! commands, hit tests), copies each frame into [`frame::FrameSnapshot`], and the
//! graphics thread renders from that copy without ever touching the plugin.
//!
//! # Web
//!
//! The plugin is a native dynamic library, so web builds keep the node, the
//! renderer and the driver state, and every spine node reports that the plugin
//! is unavailable instead of loading one.

pub mod event;
pub mod frame;
pub mod job;
pub mod node;
pub mod renderer;

#[cfg(native)]
pub mod plugin;

pub use event::*;
pub use node::SpineNode;
pub use renderer::SpineRenderer;

#[cfg(native)]
pub use plugin::SpinePlugin;
