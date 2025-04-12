mod state;

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use anyhow::Result;
use moyu_core::traits::{Command, Plugin};
use moyu_core::utils::convert::{create_promise, from_js, to_js, JSValue};
use moyu_pal::fs::{
    read_from_appdata, readdir_from_appdata, remove_from_appdata, write_to_appdata,
};
use moyu_pal::sync::Mutex;
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
    state: Arc<Mutex<ScenarioState>>,
    global_data: HashMap<String, serde_json::Value>,
}

impl ScenarioPlugin {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ScenarioState::default())),
            global_data: HashMap::new(),
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        let Some(global_data) = read_from_appdata("global_data.json").await? else {
            log::info!("global data not found, use default");
            return Ok(());
        };

        let global_data = match serde_json::from_slice(&global_data) {
            Ok(v) => v,
            Err(err) => {
                log::error!("Failed to parse global data: {}, use default", err);
                return Ok(());
            }
        };

        self.global_data = global_data;

        log::info!("global data loaded");

        Ok(())
    }

    fn next_line(&mut self) -> Option<&'static str> {
        let mut state = self.state.lock();
        let line = FAKE_SCENARIO.get(state.current_line);
        state.current_line += 1;
        line.copied()
    }

    fn save_global_data_to_file(&self) -> Result<JSValue> {
        let data = serde_json::to_vec(&self.global_data)?;
        let promise = create_promise(async move {
            let ret = write_to_appdata("global_data.json", data).await;
            log::info!("save global data to file: {:?}", ret);
            ret
        })?;
        Ok(promise)
    }

    fn save_game_data_to_file(&self, name: &str) -> Result<JSValue> {
        let data = serde_json::to_vec(&*self.state.lock())?;
        let path = format!("saves/{}.json", name);
        let promise = create_promise(async move {
            let ret = write_to_appdata(&path, data).await;
            log::info!("save game data to file: {:?}", path);
            ret
        })?;
        Ok(promise)
    }

    fn get_save_data_list(&self) -> Result<JSValue> {
        create_promise(readdir_from_appdata("saves"))
    }

    fn remove_save_data(&self, name: &str) -> Result<JSValue> {
        let path = format!("saves/{}.json", name);
        let promise = create_promise(async move {
            let ret = remove_from_appdata(&path).await;
            log::info!("remove save data: {:?}", ret);
            ret
        })?;
        Ok(promise)
    }

    fn load_save_data_from_file(&mut self, name: &str) -> Result<JSValue> {
        let path = format!("saves/{}.json", name);
        let state = self.state.clone();
        let future = async move {
            let Some(data) = read_from_appdata(&path).await? else {
                log::info!("No save data found for {}", path);
                return Ok(());
            };
            *state.lock() = serde_json::from_slice(&data)?;
            log::info!("Loaded game data from file: {}", path);
            Ok::<(), anyhow::Error>(())
        };

        let promise = create_promise(future).unwrap();

        Ok(promise)
    }

    fn start_from_uri(&self, uri: &str) {
        log::info!("Starting scenario from URI: {}", uri);
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
    StartFromUri {
        uri: String,
    },
    NextLine,
    SaveGameData {
        name: String,
        data: HashMap<String, serde_json::Value>,
    },
    LoadSaveData {
        name: String,
    },
    RemoveSaveData {
        name: String,
    },
    GetSaveDataList,
    SaveGlobalData {
        data: HashMap<String, serde_json::Value>,
    },
    GetGlobalData,
}

impl Command for ScenarioPlugin {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let payload: ScenarioCommmad = from_js(payload)?;
        log::info!("scenario plugin received: {:?}", payload);

        match payload {
            ScenarioCommmad::StartFromUri { uri } => {
                log::info!("start from uri: {}", uri);
            }
            ScenarioCommmad::NextLine => {
                return Ok(self.next_line().map(|v| to_js(&v).ok()).flatten());
            }
            ScenarioCommmad::SaveGameData { name, data } => {
                self.state.lock().extra_data = data;
                self.save_game_data_to_file(&name)?;
            }
            ScenarioCommmad::LoadSaveData { name } => {
                let result = self.load_save_data_from_file(&name)?;
                return Ok(Some(result));
            }
            ScenarioCommmad::RemoveSaveData { name } => {
                let result = self.remove_save_data(&name)?;
                return Ok(Some(result));
            }
            ScenarioCommmad::GetSaveDataList => {
                let value = self.get_save_data_list()?;
                return Ok(Some(value));
            }
            ScenarioCommmad::SaveGlobalData { data } => {
                self.global_data = data;
                self.save_global_data_to_file()?;
            }
            ScenarioCommmad::GetGlobalData => {
                return Ok(Some(to_js(&self.global_data)?));
            }
        }

        Ok(None)
    }
}
