use std::collections::HashMap;

use anyhow::Result;
use moyu_core::traits::Command;
use moyu_core::utils::convert::{JSValue, from_js, to_js};
use serde::{Deserialize, Serialize};
use sixu::format::Literal;

use crate::ScenarioPlugin;

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "subCommand"
)]
enum ScenarioCommand {
    /// Start a scenario from a URI
    AddStory {
        name: String,
        path: String,
    },
    /// Remove a scenario by name
    RemoveStory {
        name: String,
    },
    /// Check if a scenario exists by name
    HasStory {
        name: String,
    },
    /// Get the list of all scenarios
    GetStoryList,
    /// Start a scenario by name
    StartStory {
        /// The name of the story to start
        name: String,
    },
    TerminateStory,
    /// Parse the next line of the current story
    NextLine,
    /// Set the waiting time before the next line is parsed
    SetWaiting {
        /// waiting time in milliseconds
        time: u32,
        /// whether the waiting can be skipped by user input
        skippable: bool,
    },
    /// Set a variable in current game session
    SetVariable {
        name: String,
        value: serde_json::Value,
    },
    /// Get a variable from current game session
    GetVariable {
        name: String,
    },
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
    GetPermanentVariable {
        name: String,
    },
    /// Set multiple permanent variables that will be saved across game sessions
    SetPermanentVariables {
        variables: HashMap<String, serde_json::Value>,
    },
    /// Get all permanent variables that will be saved across game sessions
    GetPermanentVariables,
    /// Clear all permanent variables
    ClearPermanentVariables,

    /// save the current game session to disk
    SaveGame {
        name: String,
        extra: Option<serde_json::Value>,
    },
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
    RemoveGame {
        name: String,
    },

    /// get the list of saved game sessions
    GetGameList {
        /// matches the name of the game session
        pattern: Option<String>,
    },
}

impl Command for ScenarioPlugin {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let payload: ScenarioCommand = from_js(payload)?;
        log::debug!("scenario plugin received: {:?}", payload);

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
            ScenarioCommand::StartStory { name } => {
                log::info!("start story: {}", name);
                self.runtime.lock().start(&name)?;
                return Ok(None);
            }
            ScenarioCommand::TerminateStory => {
                log::info!("terminate story");
                self.runtime.lock().terminate()?;
                return Ok(None);
            }
            ScenarioCommand::NextLine => {
                self.next_line()?;
                return Ok(None);
            }
            ScenarioCommand::SetWaiting { time, skippable } => {
                self.set_waiting(time, skippable);
                return Ok(None);
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
                {
                    let mut runtime = self.runtime.lock();
                    runtime
                        .context_mut()
                        .global_variables_mut()
                        .as_object_mut()?
                        .insert(name, value.into());
                }
                return self.save_global_data_to_file().map(Some);
            }
            ScenarioCommand::GetPermanentVariable { name } => {
                let runtime = self.runtime.lock();
                let value = runtime.context().global_variables().as_object()?.get(&name);
                return Ok(Some(to_js(&value)?));
            }
            ScenarioCommand::SetPermanentVariables { variables } => {
                {
                    let mut runtime = self.runtime.lock();
                    let ctx = runtime.context_mut();

                    let variables = variables
                        .into_iter()
                        .map(|(k, v)| (k, v.into()))
                        .collect::<HashMap<String, Literal>>();

                    ctx.global_variables_mut()
                        .as_object_mut()?
                        .extend(variables);
                }
                return self.save_global_data_to_file().map(Some);
            }
            ScenarioCommand::GetPermanentVariables => {
                let runtime = self.runtime.lock();
                let global_vars = runtime.context().global_variables();
                return Ok(Some(to_js(&global_vars)?));
            }
            ScenarioCommand::ClearPermanentVariables => {
                {
                    let mut runtime = self.runtime.lock();
                    runtime
                        .context_mut()
                        .global_variables_mut()
                        .as_object_mut()?
                        .clear();
                }
                return self.save_global_data_to_file().map(Some);
            }
            ScenarioCommand::SaveGame { name, extra } => {
                return self.save_game_data_to_file(&name, extra).map(Some);
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
