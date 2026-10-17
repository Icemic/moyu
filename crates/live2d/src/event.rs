//! Events and metadata exposed to the project.
//!
//! The event shapes are the public API of the node: they mirror the tables in
//! the Live2D RFC and are exported to TypeScript through `ts-rs`.

use moyu_core::traits::Event;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Everything the `ready` event reports about a loaded model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct Live2dMetadata {
    pub canvas: Live2dCanvas,
    pub motions: Vec<Live2dMotion>,
    pub expressions: Vec<Live2dExpression>,
    pub hit_areas: Vec<String>,
    pub parameters: Vec<Live2dParameter>,
}

/// Canvas geometry of a model, in canvas pixels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct Live2dCanvas {
    pub width: f32,
    pub height: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub pixels_per_unit: f32,
}

/// One motion of the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct Live2dMotion {
    pub group: String,
    pub index: i32,
    pub duration: f32,
    pub fade_in: f32,
    pub fade_out: f32,
    /// Sound file path relative to the model directory; absent when the motion
    /// carries no sound.
    #[ts(optional)]
    pub sound: Option<String>,
}

/// One expression of the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct Live2dExpression {
    pub name: String,
    pub fade_in: f32,
    pub fade_out: f32,
}

/// One model parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct Live2dParameter {
    pub id: String,
    /// Parameter type: -1 unknown, 0 normal, 1 blend shape.
    pub kind: i32,
    pub min: f32,
    pub max: f32,
    #[serde(rename = "default")]
    pub default_value: f32,
}

/// Events emitted by the `live2d` node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub enum Live2dEvent {
    /// The model finished loading and the metadata below is available.
    Ready(Live2dMetadata),
    /// A motion finished playing.
    #[serde(rename_all = "camelCase")]
    MotionFinished { group: String, index: i32 },
    /// A motion fired an embedded event.
    MotionEvent { value: String },
    /// A drawable uses a blend mode outside the supported set; reported once
    /// when it first appears.
    BlendModeUnsupported { value: i32 },
    /// The plugin is unavailable, a model failed to load, or a runtime call
    /// failed.
    Error { code: i32, message: String },
}

impl Event for Live2dEvent {
    fn name(&self) -> &'static str {
        match self {
            Live2dEvent::Ready(_) => "ready",
            Live2dEvent::MotionFinished { .. } => "motionFinished",
            Live2dEvent::MotionEvent { .. } => "motionEvent",
            Live2dEvent::BlendModeUnsupported { .. } => "blendModeUnsupported",
            Live2dEvent::Error { .. } => "error",
        }
    }
}
