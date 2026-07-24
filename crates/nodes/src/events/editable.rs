use moyu_core::traits::Event;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::nodes::{EditableChangeSource, EditableState};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
#[ts(export, optional_fields)]
pub enum EditableEvent {
    Focus {
        state: EditableState,
    },
    Blur {
        state: EditableState,
    },
    Input {
        state: EditableState,
    },
    Change {
        state: EditableState,
        source: EditableChangeSource,
    },
    CompositionStart {
        state: EditableState,
    },
    CompositionUpdate {
        state: EditableState,
    },
    CompositionEnd {
        state: EditableState,
    },
}

impl Event for EditableEvent {
    fn name(&self) -> &'static str {
        match self {
            Self::Focus { .. } => "focus",
            Self::Blur { .. } => "blur",
            Self::Input { .. } => "input",
            Self::Change { .. } => "change",
            Self::CompositionStart { .. } => "compositionStart",
            Self::CompositionUpdate { .. } => "compositionUpdate",
            Self::CompositionEnd { .. } => "compositionEnd",
        }
    }
}
