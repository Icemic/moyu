use moyu_core::traits::Event;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TextLayoutEvent {
    pub text: String,
    pub width: u32,
    pub height: u32,
    pub end_cursor_position: (f32, f32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
// #[ts(export, optional_fields)]
pub enum TextEvent {
    Start,
    Progress(f64),
    Finish,
    Layout(TextLayoutEvent),
}

impl Event for TextEvent {
    fn name(&self) -> &'static str {
        match self {
            TextEvent::Start => "start",
            TextEvent::Progress(_) => "progress",
            TextEvent::Finish => "finish",
            TextEvent::Layout(_) => "textLayout",
        }
    }
}
