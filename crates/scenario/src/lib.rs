mod state;

use std::sync::LazyLock;

use anyhow::Result;
use doufu_core::traits::{Command, Plugin};
use doufu_core::utils::convert::{from_js, to_js, JSValue};
use serde::{Deserialize, Serialize};

use crate::state::ScenarioState;

static FAKE_SCENARIO: LazyLock<Vec<&'static str>> = std::sync::LazyLock::new(|| {
    vec![
        "print 这是第一行",
        "print 这是第二行",
        "print 这是第三行这是第三行这是第三行这是第三行这是第三行这是第三行",
        "show character 1",
        "print 角色 1 出现",
        "darken character 1",
        "show character 2",
        "print 角色 2 出现",
        "darken character 2",
        "show character 1",
        "print 结束了结束了结束了",
    ]
});

pub struct ScenarioPlugin {
    state: ScenarioState,
}

impl ScenarioPlugin {
    pub fn new() -> Self {
        Self {
            state: ScenarioState::default(),
        }
    }

    fn next_line(&mut self) -> Option<&'static str> {
        let line = FAKE_SCENARIO.get(self.state.current_line);
        self.state.current_line += 1;
        line.copied()
    }
}

impl Plugin for ScenarioPlugin {
    fn plugin_name(&self) -> &'static str {
        "scenario"
    }

    fn as_command(&mut self) -> Option<&mut dyn Command> {
        Some(self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "subCommand"
)]
enum ScenarioCommmad {
    StartFromUri { uri: String },
    LoadState { state: ScenarioState },
    SaveState { state: ScenarioState },
    NextLine,
}

impl Command for ScenarioPlugin {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let payload: ScenarioCommmad = from_js(payload)?;
        log::info!("scenario plugin received: {:?}", payload);

        match payload {
            ScenarioCommmad::StartFromUri { uri } => {
                log::info!("start from uri: {}", uri);
            }
            ScenarioCommmad::LoadState { state } => {
                log::info!("load state: {:?}", state);
            }
            ScenarioCommmad::SaveState { state } => {
                log::info!("save state: {:?}", state);
            }
            ScenarioCommmad::NextLine => {
                return Ok(self.next_line().map(|v| to_js(&v).ok()).flatten());
            }
        }

        Ok(None)
    }
}
