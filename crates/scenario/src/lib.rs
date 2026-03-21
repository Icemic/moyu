mod commands;
mod executor;
mod types;
mod utils;

use std::io::{Cursor, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Result;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use moyu_core::base::ImageFormat;
use moyu_core::core::get_core;
use moyu_core::plugins::SystemPlugin;
use moyu_core::traits::PluginBaseTrait;
use moyu_core::traits::{Command, Plugin, PluginEventSource};
use moyu_core::utils::convert::{JSValue, create_promise, to_js};
use moyu_macros::Plugin;
use moyu_pal::dir::assets_dir;
use moyu_pal::fs::{
    read, read_from_appdata, readdir_from_appdata, remove_from_appdata, write_to_appdata,
};
use moyu_pal::sync::Mutex;
use moyu_pal::sync::mpsc::{Receiver, Sender};
use sixu::format::RValue;
use sixu::runtime::{Runtime, StepResult};
use zip::write::SimpleFileOptions;

use crate::executor::ScenarioExecutor;
use crate::types::{GameData, ScenarioEvent, WaitingState};
use crate::utils::convert_to_literal;

const METADATA_VERSION: u32 = 1;
const ZIP_COMMENT: &str = "MOYU\0";

#[derive(Plugin)]
pub struct ScenarioPlugin {
    /// The runtime that handles scenario execution
    runtime: Arc<Mutex<Runtime<ScenarioExecutor>>>,
    /// Sender channel for execution results
    pub sender: Sender<ScenarioEvent>,
    /// Receive channel for execution results
    pub receiver: Receiver<ScenarioEvent>,
    waiting: Option<WaitingState>,
    /// Disable reacting to next line requests
    disable_next_line: Arc<AtomicBool>,
}

impl ScenarioPlugin {
    pub fn new() -> Self {
        use sixu::runtime::RuntimeContext;

        let context = RuntimeContext::new();

        let (sender, receiver) = moyu_pal::sync::mpsc::channel(100);

        let executor = ScenarioExecutor::new(sender.clone());
        let runtime = Runtime::new_with_context(executor, context);

        Self {
            runtime: Arc::new(Mutex::new(runtime)),
            sender,
            receiver,
            waiting: Default::default(),
            disable_next_line: Arc::new(AtomicBool::new(false)),
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

    fn add_story(&self, name: &str) -> Result<JSValue> {
        if self.has_story(name) {
            log::warn!("Story '{}' already exists, skipping load", name);
            return Ok(to_js(&false)?);
        }

        let runtime = self.runtime.clone();
        let name = name.to_string();
        let future = async move {
            let asset_full_path = assets_dir()
                .join(&format!("scenario/{}.sixu", name))
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to construct asset path for scenario {}: {}",
                        name,
                        e
                    )
                })?;
            let data = read(&asset_full_path).await?;
            let mut runtime = runtime.lock();
            runtime.provide_story_data(&name, data)?;
            Ok::<(), anyhow::Error>(())
        };

        let promise = match create_promise(future) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to create promise for adding story: {}", e);
                return Err(e);
            }
        };

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

    fn has_story(&self, name: &str) -> bool {
        let runtime = self.runtime.lock();
        runtime.has_story(name)
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
        let path = format!("saves/{}.sav", name);
        let runtime = self.runtime.clone();
        let future = async move {
            let Some(data) = read_from_appdata(&path).await? else {
                log::info!("No save data found for {}", path);
                return Ok(());
            };

            let mut zip = zip::ZipArchive::new(Cursor::new(data))?;

            if zip.comment() != ZIP_COMMENT.as_bytes() {
                log::warn!("Invalid ZIP comment, this may not be a valid MOYU save file");
            }

            let game_data = zip.by_name("game_data.json")?;

            let data: GameData = serde_json::from_reader(game_data)?;

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

    fn save_game_data_to_file(
        &self,
        name: &str,
        extra: Option<serde_json::Value>,
    ) -> Result<JSValue> {
        let zip_data = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(zip_data);

        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Zstd);

        {
            let runtime = self.runtime.lock();
            let stack = runtime.context().stack().clone();
            let game_vars = runtime.context().archive_variables();

            let game_data = serde_json::to_vec_pretty(&GameData {
                stack,
                variables: game_vars.clone().into(),
            })?;

            zip.start_file("game_data.json", options)?;
            zip.write_all(&game_data)?;
        }

        {
            let Some(system) = get_core().get_plugin("system") else {
                return Err(anyhow::anyhow!("System plugin is needed"));
            };

            let system = system.lock();
            let Some(system_ref): Option<&SystemPlugin> = system.as_any().downcast_ref() else {
                return Err(anyhow::anyhow!("Failed to downcast to SystemPlugin"));
            };

            if let Some(snapshot) = system_ref.snapshot().load().clone() {
                drop(system);
                let image_data = snapshot.save_to_buffer(ImageFormat::WebP)?;
                zip.start_file("snapshot.webp", options)?;
                zip.write_all(&image_data)?;
            } else {
                log::warn!("No snapshot found, skipping");
            }
        }

        {
            let timestamp = moyu_pal::time::SystemTime::now()
                .duration_since(moyu_pal::time::SystemTime::UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_millis();

            let metadata = serde_json::json!({
                "edition": METADATA_VERSION,
                "saveByVersion": env!("CARGO_PKG_VERSION"),
                "timestamp": timestamp,
                "extra": extra,
            });
            zip.start_file("metadata.json", options)?;
            zip.write_all(&serde_json::to_vec(&metadata)?)?;
        }

        {
            let extra = extra.unwrap_or(serde_json::json!({}));
            zip.start_file("extra.json", options)?;
            zip.write_all(&serde_json::to_vec(&extra)?)?;
        }

        // set identifier to detect if this is a MOYU save file
        zip.set_comment(ZIP_COMMENT);

        let zip_data = zip.finish()?.into_inner();

        let path = format!("saves/{}.sav", name);
        let promise = create_promise(async move {
            let ret = write_to_appdata(&path, zip_data).await;
            log::info!("save game data to file: {:?}", path);
            ret
        })?;
        Ok(promise)
    }

    fn remove_save_data(&self, name: &str) -> Result<JSValue> {
        let path = format!("saves/{}.sav", name);
        let promise = create_promise(async move {
            let ret = remove_from_appdata(&path).await;
            log::info!("remove save data: {:?}", ret);
            ret
        })?;
        Ok(promise)
    }

    fn get_save_data_list(&self, pattern: Option<String>) -> Result<JSValue> {
        let future = async move {
            let list = readdir_from_appdata(
                "saves",
                pattern.map(|mut p| {
                    p.push_str(".sav");
                    p
                }),
            )
            .await?;

            let mut results = Vec::new();

            for item in list.into_iter() {
                if item.is_dir {
                    continue;
                }

                let path = format!("saves/{}", item.name);
                let Some(data) = read_from_appdata(&path).await? else {
                    log::error!("No save data found for {}, this should not happen", path);
                    continue;
                };

                let mut zip = zip::ZipArchive::new(Cursor::new(data))?;
                if zip.comment() != ZIP_COMMENT.as_bytes() {
                    log::warn!("Invalid ZIP comment, this may not be a valid MOYU save file");
                }

                let snapshot = if let Ok(mut snapshot) = zip.by_name("snapshot.webp") {
                    let mut buf = Vec::with_capacity(snapshot.size() as usize);
                    // implicitly ignore errors
                    snapshot.read_to_end(&mut buf)?;

                    if snapshot.size() > 128 * 1024 {
                        // use `saves:`` schema if the image is too large
                        // add random query to avoid caching
                        Some(format!(
                            "saves:{}?random={}#snapshot.webp",
                            item.name,
                            snapshot.crc32()
                        ))
                    } else {
                        // use data URI schema directly
                        let mut prefix = "data:image/webp;base64,".to_string();
                        prefix += &BASE64_STANDARD.encode(&buf);
                        Some(prefix)
                    }
                } else {
                    None
                };

                let metadata = zip.by_name("metadata.json")?;
                let metadata: serde_json::Value = serde_json::from_reader(metadata)?;

                // extra field is optional and usually a map but not guaranteed
                let extra = if let Ok(extra) = zip.by_name("extra.json") {
                    Some(serde_json::from_reader::<_, serde_json::Value>(extra)?)
                } else {
                    None
                };

                results.push(serde_json::json!({
                    "name": item.name.trim_end_matches(".sav"),
                    "snapshot": snapshot,
                    "metadata": metadata,
                    "extra": extra,
                }));
            }

            Ok(results)
        };

        create_promise(future)
    }

    pub fn set_waiting(&mut self, time: u32, skippable: bool) {
        if self.waiting.is_some() {
            log::warn!("Already in waiting state, clearing previous timeout");
        }

        let until = moyu_pal::time::Instant::now() + Duration::from_millis(time as u64);
        self.waiting = Some(WaitingState { until, skippable });
    }

    /// Execute the next step in the scenario
    pub fn next_line(&mut self) -> Result<JSValue> {
        if self.disable_next_line.load(Ordering::Relaxed) {
            return to_js(&());
        }

        if let Some(state) = self.waiting.as_ref() {
            if state.skippable {
                let _ = self.waiting.take();
                self.send_event(ScenarioEvent::WaitingCancelled);
            } else {
                self.send_event(ScenarioEvent::Waiting);
                return to_js(&());
            }
        }

        self.disable_next_line.store(true, Ordering::Relaxed);

        let runtime = self.runtime.clone();
        let disable_next_line = self.disable_next_line.clone();
        let future = async move {
            match run_step_loop(&runtime).await {
                Ok(_) => log::info!("Scenario execution finished"),
                Err(err) => log::error!("Error during scenario execution: {}", err),
            }
            disable_next_line.store(false, Ordering::Relaxed);
            Ok::<(), anyhow::Error>(())
        };

        create_promise(future)
    }
}

/// Run the step/resume loop for scenario execution.
///
/// Acquires the runtime lock only for each `step()` call and releases it
/// before performing async operations (eval_in_sandbox), preventing deadlocks
/// when JS callbacks (e.g. setVariable) re-enter the scenario plugin.
async fn run_step_loop(
    runtime: &Arc<Mutex<Runtime<ScenarioExecutor>>>,
) -> std::result::Result<(), anyhow::Error> {
    use moyu_core::utils::eval_in_sandbox::eval_in_sandbox;
    use moyu_pal::dir::assets_dir;

    loop {
        // Acquire lock, run one synchronous step, then release lock
        let step_result = {
            let mut rt = runtime.lock();
            rt.step().map_err(anyhow::Error::from)
        }; // lock released here

        match step_result? {
            StepResult::Done => return Ok(()),
            StepResult::NeedsCondition(condition) => {
                // Evaluate condition outside the lock
                let ret = eval_in_sandbox(format!("Boolean({})", condition)).await?;
                let result = ret.as_bool().unwrap_or(false);
                // Re-acquire lock to provide result
                let mut rt = runtime.lock();
                rt.resume_condition(result);
            }
            StepResult::NeedsScript(script) => {
                // Evaluate script outside the lock
                let ret = eval_in_sandbox(format!("(() => {{ {} }})()", script)).await?;
                let literal = convert_to_literal(ret);
                // Re-acquire lock to provide result
                let mut rt = runtime.lock();
                rt.resume_script(Some(RValue::Literal(literal)), true);
            }
            StepResult::NeedsStoryFile(story_name) => {
                // Load story file outside the lock
                let asset_full_path = assets_dir()
                    .join(&format!("scenario/{}.sixu", story_name))
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to construct asset path for scenario {}: {}",
                            story_name,
                            e
                        )
                    })?;
                let data = moyu_pal::fs::read(&asset_full_path).await?;
                log::info!("Loaded scenario from file: {}", asset_full_path);
                // Re-acquire lock to provide data
                let mut rt = runtime.lock();
                rt.provide_story_data(&story_name, data)
                    .map_err(anyhow::Error::from)?;
            }
        }
    }
}

impl Plugin for ScenarioPlugin {
    fn update(&mut self, _: bool) {
        if let Some(state) = self.waiting.as_ref() {
            if moyu_pal::time::Instant::now() >= state.until {
                let _ = self.waiting.take();

                let runtime = self.runtime.clone();
                moyu_pal::task::spawn(async move {
                    if let Err(err) = run_step_loop(&runtime).await {
                        log::error!("Error during scenario execution after wait: {}", err);
                    }
                });
            }
        }

        if let Ok(event) = self.receiver.try_recv() {
            self.send_event(event);
        }
    }

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
