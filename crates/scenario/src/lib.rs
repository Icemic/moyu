mod events;
mod executor;
mod state;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use moyu_core::traits::{Command, Plugin, PluginEventSource};
use moyu_core::utils::convert::{create_promise, from_js, to_js, JSValue};
use moyu_pal::config::entry_dir;
use moyu_pal::fs::{
    read_from_appdata, readdir_from_appdata, remove_from_appdata, write_to_appdata,
};
use moyu_pal::sync::mpsc::error::TryRecvError;
use moyu_pal::sync::mpsc::Receiver;
use moyu_pal::sync::Mutex;
use serde::{Deserialize, Serialize};
use sixu::format::Literal;
use sixu::runtime::{ExecutionState, Runtime};

use crate::events::ScenarioEvent;
use crate::executor::{ExecutionResult, ScenarioExecutor};

pub struct ScenarioPlugin {
    /// The runtime that handles scenario execution
    runtime: Arc<Mutex<Runtime<ScenarioExecutor>>>,
    /// Receive channel for execution results
    pub receiver: Receiver<ExecutionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameData {
    /// The current execution state stack
    pub stack: Vec<ExecutionState>,
    /// Variables for the current game session
    pub variables: serde_json::Value,
}

impl ScenarioPlugin {
    pub fn new() -> Self {
        use sixu::runtime::RuntimeContext;

        let context = RuntimeContext::new();

        let (sender, receiver) = moyu_pal::sync::mpsc::channel(1);

        let executor = ScenarioExecutor::new(sender);
        let runtime = Runtime::new_with_context(executor, context);

        Self {
            runtime: Arc::new(Mutex::new(runtime)),
            receiver,
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

        // Set global variables directly in runtime context (no conversion needed)
        let mut runtime = self.runtime.lock();
        let ctx = runtime.context_mut();
        *ctx.global_variables_mut() = global_data;

        log::info!("global data loaded");

        Ok(())
    }

    fn add_story(&self, name: &str, path: &str) -> Result<JSValue> {
        if self.has_story(name)? {
            log::warn!("Story '{}' already exists, skipping load", name);
            return Ok(to_js(&false)?);
        }

        let asset_full_path = entry_dir().join("assets/").unwrap().join(path).unwrap();

        let name = name.to_string();
        let runtime = self.runtime.clone();
        let future = async move {
            let data = match moyu_pal::fs::read(&asset_full_path).await {
                Ok(data) => data,
                Err(e) => {
                    log::error!(
                        "Failed to read font file: {}, text rendering may not work.",
                        e
                    );
                    return Err(e.into());
                }
            };
            let text = String::from_utf8(data)
                .map_err(|e| anyhow::anyhow!("Failed to parse story file: {}", e))?;

            let (_, story) = sixu::parser::parse(&name, &text).unwrap();

            runtime.lock().context_mut().stories_mut().push(story);

            log::info!("Loaded game data from file: {}", asset_full_path);
            Ok::<(), anyhow::Error>(())
        };

        let promise = create_promise(future).unwrap();

        Ok(promise)
    }

    fn remove_story(&self, name: &str) -> Result<JSValue> {
        let mut runtime = self.runtime.lock();
        let stories = runtime.context_mut().stories_mut();
        if let Some(pos) = stories.iter().position(|s| s.name == name) {
            stories.remove(pos);
            log::info!("Removed story: {}", name);
            return Ok(to_js(&true)?);
        }
        log::warn!("Story '{}' not found, cannot remove", name);
        Ok(to_js(&false)?)
    }

    fn has_story(&self, name: &str) -> Result<bool> {
        let runtime = self.runtime.lock();
        let stories = runtime.context().stories();
        Ok(stories.iter().any(|s| s.name == name))
    }

    fn get_story_list(&self) -> Result<JSValue> {
        let runtime = self.runtime.lock();
        let stories = runtime.context().stories();
        let story_names: Vec<String> = stories.iter().map(|s| s.name.clone()).collect();
        Ok(to_js(&story_names)?)
    }

    fn save_global_data_to_file(&self) -> Result<JSValue> {
        // Get global variables from runtime context (already in serde_json::Value format)
        let runtime = self.runtime.lock();
        let ctx = runtime.context();
        let global_vars = ctx.global_variables();

        let data = serde_json::to_vec(&global_vars)?;
        let promise = create_promise(async move {
            let ret = write_to_appdata("global_data.json", data).await;
            log::info!("save global data to file: {:?}", ret);
            ret
        })?;
        Ok(promise)
    }

    fn load_save_data_from_file(&mut self, name: &str) -> Result<JSValue> {
        let path = format!("saves/{}.json", name);
        let runtime = self.runtime.clone();
        let future = async move {
            let Some(data) = read_from_appdata(&path).await? else {
                log::info!("No save data found for {}", path);
                return Ok(());
            };
            let data: GameData = serde_json::from_slice(&data)?;

            // Update the runtime's context
            {
                let mut runtime = runtime.lock();
                let context = runtime.context_mut();
                *context.stack_mut() = data.stack;
                *context.archive_variables_mut() = data.variables.into();
            }

            log::info!("Loaded game data from file: {}", path);
            Ok::<(), anyhow::Error>(())
        };

        let promise = create_promise(future).unwrap();

        Ok(promise)
    }

    fn save_game_data_to_file(&self, name: &str) -> Result<JSValue> {
        let runtime = self.runtime.lock();
        let stack = runtime.context().stack().clone();
        let game_vars = runtime.context().archive_variables();

        let data = serde_json::to_vec_pretty(&GameData {
            stack,
            variables: game_vars.clone().into(),
        })?;

        let path = format!("saves/{}.json", name);
        let promise = create_promise(async move {
            let ret = write_to_appdata(&path, data).await;
            log::info!("save game data to file: {:?}", path);
            ret
        })?;
        Ok(promise)
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

    fn get_save_data_list(&self, _pattern_str: Option<String>) -> Result<JSValue> {
        create_promise(readdir_from_appdata("saves"))
    }

    /// Execute the next step in the scenario
    pub fn next_line(&mut self) -> Result<ExecutionResult> {
        let mut runtime = self.runtime.lock();
        loop {
            runtime.next()?;
            match self.receiver.try_recv() {
                Ok(result) => return Ok(result),
                Err(TryRecvError::Empty) => {
                    log::debug!("No execution result available, continuing execution");
                    continue;
                }
                Err(TryRecvError::Disconnected) => {
                    log::error!("Receiver disconnected, stopping execution");
                    return Err(anyhow::anyhow!("Receiver disconnected"));
                }
            }
        }
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
enum ScenarioCommand {
    /// Start a scenario from a URI
    AddStory { name: String, path: String },
    /// Remove a scenario by name
    RemoveStory { name: String },
    /// Check if a scenario exists by name
    HasStory { name: String },
    /// Get the list of all scenarios
    GetStoryList,

    /// Start a scenario from a URI
    NextLine,
    SetVariable {
        name: String,
        value: serde_json::Value,
    },
    /// Get a variable from current game session
    GetVariable { name: String },
    /// Set multiple variables to current game session
    SetVariables {
        variables: HashMap<String, serde_json::Value>,
    },
    /// Get all variables from current game session
    GetVariables,

    /// Set a permanent variable that will be saved across game sessions
    SetPermanentVariable {
        name: String,
        value: serde_json::Value,
    },
    /// Get a permanent variable that will be saved across game sessions
    GetPermanentVariable { name: String },
    /// Set multiple permanent variables that will be saved across game sessions
    SetPermanentVariables {
        variables: HashMap<String, serde_json::Value>,
    },
    /// Get all permanent variables that will be saved across game sessions
    GetPermanentVariables,
    /// Clear all permanent variables
    ClearPermanentVariables,

    /// save the current game session to disk
    SaveGame { name: String },
    /// load a saved game session from disk
    LoadGame {
        name: String,
        /// if true, will overwrite the current game session with the loaded one,
        /// otherwise an error will be returned if the current session is not empty
        overwrite: bool,
    },
    /// reset the current game session to initial state
    ResetGame,
    /// remove a saved game session from disk
    RemoveGame { name: String },

    /// get the list of saved game sessions
    GetGameList {
        /// matches the name of the game session
        pattern: Option<String>,
    },
}

impl Command for ScenarioPlugin {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let payload: ScenarioCommand = from_js(payload)?;
        log::info!("scenario plugin received: {:?}", payload);

        match payload {
            ScenarioCommand::AddStory { name, path } => {
                log::info!("add story: {} {}", name, path);
                return self.add_story(&name, &path).map(Some);
            }
            ScenarioCommand::RemoveStory { name } => {
                log::info!("remove story: {}", name);
                return self.remove_story(&name).map(Some);
            }
            ScenarioCommand::HasStory { name } => {
                log::info!("has story: {}", name);
                return Ok(Some(to_js(&self.has_story(&name)?)?));
            }
            ScenarioCommand::GetStoryList => {
                log::info!("get story list");
                return self.get_story_list().map(Some);
            }
            ScenarioCommand::NextLine => {
                let result = self.next_line()?;
                self.send_event(
                    &result.as_ref().to_lowercase(),
                    ScenarioEvent::ExecutionResult(result.clone()),
                );
                return Ok(Some(to_js(&result)?));
            }

            ScenarioCommand::SetVariable { name, value } => {
                let mut runtime = self.runtime.lock();
                runtime
                    .context_mut()
                    .archive_variables_mut()
                    .as_object_mut()?
                    .insert(name, value.into());
            }
            ScenarioCommand::GetVariable { name } => {
                let runtime = self.runtime.lock();
                let value = runtime
                    .context()
                    .archive_variables()
                    .as_object()?
                    .get(&name);
                return Ok(Some(to_js(&value)?));
            }
            ScenarioCommand::SetVariables { variables } => {
                let mut runtime = self.runtime.lock();
                let ctx = runtime.context_mut();

                let variables = variables
                    .into_iter()
                    .map(|(k, v)| (k, v.into()))
                    .collect::<HashMap<String, Literal>>();

                ctx.archive_variables_mut()
                    .as_object_mut()?
                    .extend(variables);
                return Ok(None);
            }
            ScenarioCommand::GetVariables => {
                let runtime = self.runtime.lock();
                let game_vars = runtime.context().archive_variables();
                return Ok(Some(to_js(&game_vars)?));
            }
            ScenarioCommand::SetPermanentVariable { name, value } => {
                let mut runtime = self.runtime.lock();
                runtime
                    .context_mut()
                    .global_variables_mut()
                    .as_object_mut()?
                    .insert(name, value.into());
                return self.save_global_data_to_file().map(Some);
            }
            ScenarioCommand::GetPermanentVariable { name } => {
                let runtime = self.runtime.lock();
                let value = runtime.context().global_variables().as_object()?.get(&name);
                return Ok(Some(to_js(&value)?));
            }
            ScenarioCommand::SetPermanentVariables { variables } => {
                let mut runtime = self.runtime.lock();
                let ctx = runtime.context_mut();

                let variables = variables
                    .into_iter()
                    .map(|(k, v)| (k, v.into()))
                    .collect::<HashMap<String, Literal>>();

                ctx.global_variables_mut()
                    .as_object_mut()?
                    .extend(variables);
                return self.save_global_data_to_file().map(Some);
            }
            ScenarioCommand::GetPermanentVariables => {
                let runtime = self.runtime.lock();
                let global_vars = runtime.context().global_variables();
                return Ok(Some(to_js(&global_vars)?));
            }
            ScenarioCommand::ClearPermanentVariables => {
                let mut runtime = self.runtime.lock();
                runtime
                    .context_mut()
                    .global_variables_mut()
                    .as_object_mut()?
                    .clear();
                return self.save_global_data_to_file().map(Some);
            }
            ScenarioCommand::SaveGame { name } => {
                return self.save_game_data_to_file(&name).map(Some);
            }
            ScenarioCommand::LoadGame { name, overwrite } => {
                if !overwrite && !self.runtime.lock().context().stack().is_empty() {
                    return Err(anyhow::anyhow!("Current game session is not empty"));
                }
                return self.load_save_data_from_file(&name).map(Some);
            }
            ScenarioCommand::ResetGame => {
                let mut runtime = self.runtime.lock();
                let ctx = runtime.context_mut();
                ctx.stack_mut().clear();
                ctx.archive_variables_mut().as_object_mut()?.clear();
                return Ok(None);
            }
            ScenarioCommand::RemoveGame { name } => {
                return self.remove_save_data(&name).map(Some);
            }
            ScenarioCommand::GetGameList { pattern } => {
                let value = self.get_save_data_list(pattern)?;
                return Ok(Some(value));
            }
        }

        Ok(None)
    }
}

impl PluginEventSource for ScenarioPlugin {
    type Event = ScenarioEvent;
}
