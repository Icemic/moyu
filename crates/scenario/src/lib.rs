mod commands;
mod events;
mod executor;
mod types;

use std::sync::Arc;

use anyhow::Result;
use moyu_core::traits::{Command, Plugin, PluginEventSource};
use moyu_core::utils::convert::{create_promise, to_js, JSValue};
use moyu_pal::config::entry_dir;
use moyu_pal::fs::{
    read_from_appdata, readdir_from_appdata, remove_from_appdata, write_to_appdata,
};
use moyu_pal::sync::mpsc::error::TryRecvError;
use moyu_pal::sync::mpsc::Receiver;
use moyu_pal::sync::Mutex;
use sixu::runtime::Runtime;

use crate::events::ScenarioEvent;
use crate::executor::ScenarioExecutor;
use crate::types::{ExecutionResult, GameData};

pub struct ScenarioPlugin {
    /// The runtime that handles scenario execution
    runtime: Arc<Mutex<Runtime<ScenarioExecutor>>>,
    /// Receive channel for execution results
    pub receiver: Receiver<ExecutionResult>,
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

    fn get_save_data_list(&self, pattern: Option<String>) -> Result<JSValue> {
        create_promise(async move {
            let mut list = readdir_from_appdata(
                "saves",
                pattern.map(|mut p| {
                    p.push_str(".json");
                    p
                }),
            )
            .await?;

            list.iter_mut().for_each(|entry| {
                entry.name = entry
                    .name
                    .strip_suffix(".json")
                    .unwrap_or_default()
                    .to_string();
            });

            Ok(list)
        })
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

impl PluginEventSource for ScenarioPlugin {
    type Event = ScenarioEvent;
}
