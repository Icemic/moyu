//! Events and metadata exposed to the project.
//!
//! The event shapes are the public API of the node: they mirror the tables in
//! the Spine RFC and are exported to TypeScript through `ts-rs`.

use moyu_core::traits::Event;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Everything the `ready` event reports about a loaded model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct SpineMetadata {
    /// Skeleton data version string, as written by the Spine Editor.
    ///
    /// Diagnostic only: it never selects a runtime.
    pub data_version: String,
    /// Bounds measured once at load in the setup pose, in world units.
    ///
    /// This is the layout rectangle: it does not follow animations or skin
    /// changes, and it is not a clip boundary.
    pub bounds: SpineBounds,
    pub animations: Vec<SpineAnimation>,
    pub skins: Vec<String>,
    pub pages: Vec<SpinePage>,
    pub bounding_boxes: Vec<String>,
}

/// A rectangle in world units with y up.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct SpineBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// One animation of the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct SpineAnimation {
    pub name: String,
    /// Length in seconds; zero for animations that only hold a pose.
    pub duration: f32,
}

/// One texture page of the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct SpinePage {
    /// Image path relative to the asset root.
    pub path: String,
    pub width: u32,
    pub height: u32,
    /// Whether the source alpha is already premultiplied.
    pub premultiplied: bool,
    /// How the page declares premultiplied alpha, for diagnostics.
    pub alpha_source: AlphaSource,
}

/// Where the `premultiplied` flag came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AlphaSource {
    /// The atlas declared `pma: true`.
    Atlas,
    /// The atlas declared `pma: false`; the image is premultiplied on upload.
    Image,
}

/// Identity of one playback.
///
/// `entry` is assigned by the plugin and stays unique within one loaded model;
/// `epoch` is bumped by the host on every model change. Together they survive a
/// reload: an entry id is reused after the plugin drops the old model, so a
/// waiter that only compared `entry` would match the wrong playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct EntryIdentity {
    pub entry: u64,
    pub epoch: u64,
}

/// Events emitted by the `spine` node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub enum SpineEvent {
    /// The model finished parsing and the metadata below is available.
    ///
    /// Texture pages may still be loading; the frame is only drawn once every
    /// page is ready.
    Ready(SpineMetadata),
    /// An entry began playing.
    #[serde(rename_all = "camelCase")]
    TrackStart {
        track: i32,
        id: EntryIdentity,
        animation: String,
    },
    /// An entry reached the end of its animation for this iteration.
    #[serde(rename_all = "camelCase")]
    TrackComplete {
        track: i32,
        id: EntryIdentity,
        animation: String,
    },
    /// An entry stopped because another one took over its track.
    #[serde(rename_all = "camelCase")]
    TrackEnd {
        track: i32,
        id: EntryIdentity,
        animation: String,
    },
    /// An entry is gone; the terminal signal for its identity.
    ///
    /// Fires for completion, interruption and cancellation alike, so waiting on
    /// it always terminates. A model reload also disposes every entry the old
    /// model still had, so a waiter never hangs across a reload.
    #[serde(rename_all = "camelCase")]
    TrackDispose {
        track: i32,
        id: EntryIdentity,
        animation: String,
    },
    /// An animation fired an embedded user event.
    #[serde(rename_all = "camelCase")]
    TrackEvent {
        track: i32,
        id: EntryIdentity,
        animation: String,
        name: String,
        int_value: i32,
        float_value: f32,
        string_value: String,
        audio_path: String,
        volume: f32,
        balance: f32,
    },
    /// The pointer pressed inside a named bounding box.
    ///
    /// `x` / `y` are world units; the name is the bounding box attachment. The
    /// epoch says which model was hit, so a handler holding a stale model does
    /// not act on a reload's geometry.
    #[serde(rename_all = "camelCase")]
    Hit {
        epoch: u64,
        name: String,
        x: f32,
        y: f32,
    },
    /// The plugin is unavailable, a model failed to load, or a runtime call
    /// failed.
    Error { code: i32, message: String },
}

impl Event for SpineEvent {
    fn name(&self) -> &'static str {
        match self {
            SpineEvent::Ready(_) => "ready",
            SpineEvent::TrackStart { .. } => "trackStart",
            SpineEvent::TrackComplete { .. } => "trackComplete",
            SpineEvent::TrackEnd { .. } => "trackEnd",
            SpineEvent::TrackDispose { .. } => "trackDispose",
            SpineEvent::TrackEvent { .. } => "trackEvent",
            SpineEvent::Hit { .. } => "hit",
            SpineEvent::Error { .. } => "error",
        }
    }
}
