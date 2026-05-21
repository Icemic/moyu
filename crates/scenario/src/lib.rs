mod commands;
mod execution_path;
mod executor;
mod replace_recovery;
mod story_graph;
mod types;
mod utils;

use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
use sixu::Fingerprint;
use sixu::format::RValue;
use sixu::format::{Block, ChildContent, Literal, Story};
use sixu::runtime::{ExecutionState, Runtime, StepResult};
use zip::write::SimpleFileOptions;

use crate::executor::{ScenarioExecutor, WarpState};
use crate::execution_path::capture_execution_path;
use crate::replace_recovery::plan_story_replace;
use crate::story_graph::build_story_graph;
use crate::types::{
    BacklogState, ExecutionCursor, GameData, RuntimeCheckpoint, RuntimeSnapshot, ScenarioEvent,
    ScenarioRecord, WarpBoundary, WaitingState,
};
use crate::utils::{
    convert_to_literal, next_record_id, prune_backlog_blocks, snapshot_blocks, timestamp_millis,
};

const METADATA_VERSION: u32 = 2;
const MAX_RECORDS: usize = 50;
const ZIP_COMMENT: &str = "MOYU\0";

/// Maximum number of internal steps per single run_step_loop call.
/// Prevents infinite loops caused by unconditional goto/replace/call cycles
/// that never yield a text line or command (which would otherwise deadlock
/// the async executor indefinitely).
const MAX_STEPS_PER_RUN: usize = 100_000;

#[derive(Plugin)]
pub struct ScenarioPlugin {
    /// The runtime that handles scenario execution
    runtime: Arc<Mutex<Runtime<ScenarioExecutor>>>,
    /// Sender channel for execution results
    pub sender: Sender<ScenarioEvent>,
    /// Receive channel for execution results
    pub receiver: Receiver<ScenarioEvent>,
    backlog: Arc<Mutex<BacklogState>>,
    checkpoints: Arc<Mutex<HashMap<String, RuntimeCheckpoint>>>,
    checkpoint_blocks: Arc<Mutex<HashMap<Fingerprint, Block>>>,
    current_marker_id: Arc<Mutex<Option<String>>>,
    warp_state: Arc<Mutex<Option<WarpState>>>,
    waiting: Option<WaitingState>,
    /// Disable reacting to next line requests
    disable_next_line: Arc<AtomicBool>,
    execution_generation: Arc<AtomicU64>,
}

impl ScenarioPlugin {
    pub fn new() -> Self {
        use sixu::runtime::RuntimeContext;

        let context = RuntimeContext::new();

        let (sender, receiver) = moyu_pal::sync::mpsc::channel(10000);

        let checkpoints = Arc::new(Mutex::new(HashMap::new()));
        let checkpoint_blocks = Arc::new(Mutex::new(HashMap::new()));
        let current_marker_id = Arc::new(Mutex::new(None));
        let warp_state = Arc::new(Mutex::new(None));
        let executor = ScenarioExecutor::new(
            sender.clone(),
            checkpoints.clone(),
            checkpoint_blocks.clone(),
            warp_state.clone(),
        );
        let runtime = Runtime::new_with_context(executor, context);

        Self {
            runtime: Arc::new(Mutex::new(runtime)),
            sender,
            receiver,
            backlog: Arc::new(Mutex::new(BacklogState::default())),
            checkpoints,
            checkpoint_blocks,
            current_marker_id,
            warp_state,
            waiting: Default::default(),
            disable_next_line: Arc::new(AtomicBool::new(false)),
            execution_generation: Arc::new(AtomicU64::new(0)),
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

    fn clear_backlog(&self) {
        let mut backlog = self.backlog.lock();
        backlog.records.clear();
        backlog.blocks.clear();
        backlog.next_record_serial = 0;
    }

    fn clear_checkpoints(&self) {
        self.checkpoints.lock().clear();
        self.checkpoint_blocks.lock().clear();
    }

    fn drain_pending_events(&mut self) {
        while self.receiver.try_recv().is_ok() {}
    }

    fn begin_runtime_transition(&mut self) -> u64 {
        let generation = self.execution_generation.fetch_add(1, Ordering::Relaxed) + 1;
        self.waiting = None;
        self.disable_next_line.store(false, Ordering::Relaxed);
        *self.warp_state.lock() = None;
        self.drain_pending_events();
        generation
    }

    fn reset_debug_state(&self) {
        self.clear_checkpoints();
        *self.current_marker_id.lock() = None;
    }

    fn build_execution_cursor(&self) -> Result<Option<ExecutionCursor>> {
        let runtime = self.runtime.lock();
        let Some(current_state) = runtime.context().stack().last() else {
            return Ok(None);
        };

        Ok(Some(ExecutionCursor {
            story: current_state.story.clone(),
            paragraph: current_state.paragraph.clone(),
            marker_id: self.current_marker_id.lock().clone(),
        }))
    }

    fn get_execution_cursor(&self) -> Result<JSValue> {
        to_js(&self.build_execution_cursor()?)
    }

    fn collect_block_markers(block: &Block, markers: &mut Vec<String>) {
        for child in block.children() {
            if let Some(marker) = &child.marker {
                markers.push(marker.id.clone());
            }

            if let ChildContent::Block(block) = &child.content {
                Self::collect_block_markers(block, markers);
            }
        }
    }

    fn collect_story_markers(story: &Story) -> Vec<String> {
        let mut markers = Vec::new();

        for paragraph in &story.paragraphs {
            Self::collect_block_markers(&paragraph.block, &mut markers);
        }

        markers
    }

    fn get_fast_forward_checkpoint(&self, key: &str) -> Result<JSValue> {
        let (marker_exists, checkpoint_key) = self.get_fast_forward_checkpoint_info(key)?;
        to_js(&serde_json::json!({
            "markerExists": marker_exists,
            "checkpointKey": checkpoint_key,
        }))
    }

    fn get_fast_forward_checkpoint_info(&self, key: &str) -> Result<(bool, Option<String>)> {
        let Some(cursor) = self.build_execution_cursor()? else {
            return Ok((false, None));
        };

        let markers = {
            let runtime = self.runtime.lock();
            let Some(story) = runtime
                .context()
                .stories()
                .iter()
                .find(|story| story.name == cursor.story)
            else {
                return Ok((false, None));
            };

            Self::collect_story_markers(story)
        };

        let Some(target_index) = markers.iter().position(|marker_id| marker_id == key) else {
            return Ok((false, None));
        };

        let checkpoints = self.checkpoints.lock();
        for marker_id in markers[..target_index].iter().rev() {
            let Some(checkpoint) = checkpoints.get(marker_id) else {
                continue;
            };
            let Some(checkpoint_cursor) = checkpoint.cursor.as_ref() else {
                continue;
            };
            if checkpoint_cursor.story == cursor.story {
                return Ok((true, Some(marker_id.clone())));
            }
        }

        Ok((true, None))
    }

    #[cfg(test)]
    fn capture_checkpoint_state(&mut self, key: &str) -> Result<()> {
        let snapshot = {
            let runtime = self.runtime.lock();
            let mut checkpoint_blocks = self.checkpoint_blocks.lock();
            crate::utils::create_runtime_snapshot_from_context(
                runtime.context(),
                &mut checkpoint_blocks,
            )?
        };
        let cursor = self.build_execution_cursor()?;

        self.checkpoints
            .lock()
            .insert(key.to_string(), RuntimeCheckpoint { cursor, snapshot });

        Ok(())
    }

    fn restore_checkpoint(&mut self, key: &str) -> Result<JSValue> {
        to_js(&self.restore_checkpoint_state(key)?)
    }

    fn restore_checkpoint_state(&mut self, key: &str) -> Result<bool> {
        let Some(checkpoint) = self.checkpoints.lock().get(key).cloned() else {
            return Ok(false);
        };

        self.begin_runtime_transition();
        {
            let checkpoint_blocks = self.checkpoint_blocks.lock();
            Self::restore_runtime_snapshot(
                &self.runtime,
                &checkpoint.snapshot,
                &checkpoint_blocks,
            )?;
        }
        *self.current_marker_id.lock() = checkpoint.cursor.and_then(|cursor| cursor.marker_id);

        Ok(true)
    }
    fn track_cursor_event(&self, event: &ScenarioEvent) {
        match event {
            ScenarioEvent::MarkerEnter(marker) => {
                *self.current_marker_id.lock() = Some(marker.marker_id.clone());
            }
            ScenarioEvent::Finished => {
                *self.current_marker_id.lock() = None;
            }
            _ => {}
        }
    }

    fn take_queued_event(&mut self) -> Option<ScenarioEvent> {
        let event = self.receiver.try_recv().ok()?;
        self.track_cursor_event(&event);
        Some(event)
    }

    fn create_runtime_snapshot(
        &self,
    ) -> Result<(RuntimeSnapshot, HashMap<Fingerprint, Block>)> {
        let mut blocks = HashMap::new();
        let runtime = self.runtime.lock();
        let snapshot =
            crate::utils::create_runtime_snapshot_from_context(runtime.context(), &mut blocks)?;
        Ok((snapshot, blocks))
    }

    fn restore_runtime_snapshot(
        runtime: &Arc<Mutex<Runtime<ScenarioExecutor>>>,
        snapshot: &RuntimeSnapshot,
        blocks: &HashMap<Fingerprint, Block>,
    ) -> Result<()> {
        let stack = snapshot
            .stack
            .iter()
            .map(|state| {
                let block = blocks
                    .get(&state.block_fingerprint)
                    .cloned()
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Block fingerprint {} not found in backlog block pool",
                            state.block_fingerprint.to_hex()
                        )
                    })?;

                let locals = match &state.locals {
                    Some(value) => {
                        let literal = Literal::from(value.clone());
                        match literal {
                            Literal::Object(map) => Some(map),
                            other => {
                                return Err(anyhow::anyhow!(
                                    "Saved locals for {}::{} must be an object, got {}",
                                    state.story,
                                    state.paragraph,
                                    other.to_string()
                                ));
                            }
                        }
                    }
                    None => None,
                };

                Ok(ExecutionState {
                    story: state.story.clone(),
                    paragraph: state.paragraph.clone(),
                    block,
                    index: state.index,
                    is_loop_body: state.is_loop_body,
                    locals,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let mut runtime = runtime.lock();
        let context = runtime.context_mut();
        *context.stack_mut() = stack;
        *context.archive_variables_mut() = snapshot.variables.clone().into();

        Ok(())
    }

    fn build_game_data(&self) -> Result<GameData> {
        let (current_state, current_state_blocks) = self.create_runtime_snapshot()?;
        let (records, backlog_blocks) = {
            let backlog = self.backlog.lock();
            (backlog.records.clone(), backlog.blocks.clone())
        };

        let mut referenced = records
            .iter()
            .flat_map(|record| snapshot_blocks(&record.snapshot))
            .collect::<HashSet<_>>();
        referenced.extend(snapshot_blocks(&current_state));

        let mut blocks = backlog_blocks
            .into_iter()
            .filter(|(fingerprint, _)| referenced.contains(fingerprint))
            .collect::<HashMap<_, _>>();
        blocks.extend(current_state_blocks);

        Ok(GameData {
            current_state,
            records,
            blocks,
        })
    }

    fn record(&mut self, meta: HashMap<String, serde_json::Value>) -> Result<JSValue> {
        let (snapshot, blocks) = self.create_runtime_snapshot()?;

        let mut backlog = self.backlog.lock();
        backlog.blocks.extend(blocks);

        let id = next_record_id(&mut backlog)?;
        let created_at = timestamp_millis()?;

        backlog.records.push(ScenarioRecord {
            id: id.clone(),
            created_at,
            meta,
            snapshot,
        });

        if backlog.records.len() > MAX_RECORDS {
            backlog.records.remove(0);
        }

        prune_backlog_blocks(&mut backlog);

        to_js(&id)
    }

    fn get_records(&self, offset: Option<usize>, limit: Option<usize>) -> Result<JSValue> {
        let backlog = self.backlog.lock();
        let offset = offset.unwrap_or(0);

        let iter = backlog.records.iter().rev().skip(offset);
        let records = match limit {
            Some(limit) => iter.take(limit).collect::<Vec<_>>(),
            None => iter.collect::<Vec<_>>(),
        };

        let infos = records
            .into_iter()
            .map(|record| record.get_info())
            .collect::<Vec<_>>();

        to_js(&infos)
    }

    fn jump_to_record(&mut self, record_id: &str) -> Result<JSValue> {
        let (snapshot, blocks) = {
            let mut backlog = self.backlog.lock();
            let Some(index) = backlog
                .records
                .iter()
                .position(|record| record.id == record_id)
            else {
                return to_js(&false);
            };

            backlog.records.truncate(index + 1);
            prune_backlog_blocks(&mut backlog);

            let snapshot = backlog
                .records
                .last()
                .map(|record| record.snapshot.clone())
                .ok_or_else(|| anyhow::anyhow!("Target backlog record missing after truncate"))?;

            (snapshot, backlog.blocks.clone())
        };

        self.begin_runtime_transition();
        Self::restore_runtime_snapshot(&self.runtime, &snapshot, &blocks)?;
        *self.current_marker_id.lock() = None;

        to_js(&true)
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

    fn replace_story_data(&mut self, name: &str, content: &str) -> Result<JSValue> {
        let (_, next_story) = sixu::parser::parse(name, content)
            .map_err(|error| anyhow::anyhow!("Failed to parse story file '{}': {}", name, error))?;

        let existing_story = {
            let runtime = self.runtime.lock();
            runtime
                .context()
                .stories()
                .iter()
                .find(|story| story.name == name)
                .cloned()
        };

        let Some(existing_story) = existing_story else {
            let mut runtime = self.runtime.lock();
            runtime.context_mut().stories_mut().push(next_story);
            return to_js(&crate::types::StoryReplaceOutcome {
                story: name.to_string(),
                current_story_affected: false,
                plan: crate::types::StoryReplaceExecutionPlan {
                    mode: crate::types::StoryReplaceMode::InvalidateOnly,
                    boundary: None,
                    target_marker_id: None,
                    changed_control_flow: true,
                },
            });
        };

        let current_marker_id = self.current_marker_id.lock().clone();
        let (current_story_affected, execution_path) = {
            let runtime = self.runtime.lock();
            let path = capture_execution_path(runtime.context(), current_marker_id);
            let affected = path
                .frames
                .last()
                .map(|frame| frame.story == name)
                .unwrap_or(false);
            (affected, path)
        };

        let old_graph = build_story_graph(&existing_story);
        let new_graph = build_story_graph(&next_story);

        let replace_plan = {
            let checkpoints = self.checkpoints.lock();
            let checkpoint_blocks = self.checkpoint_blocks.lock();

            plan_story_replace(
                name,
                current_story_affected,
                &existing_story,
                &next_story,
                &old_graph,
                &new_graph,
                &execution_path,
                &checkpoints,
                &checkpoint_blocks,
            )?
        };

        {
            let mut runtime = self.runtime.lock();
            let stories = runtime.context_mut().stories_mut();

            if let Some(position) = stories.iter().position(|story| story.name == name) {
                stories[position] = next_story;
            } else {
                stories.push(next_story);
            }
        }

        *self.checkpoints.lock() = replace_plan.checkpoints.clone();
        *self.checkpoint_blocks.lock() = replace_plan.checkpoint_blocks.clone();

        if current_story_affected
            && !matches!(replace_plan.outcome.plan.mode, crate::types::StoryReplaceMode::Noop)
        {
            *self.current_marker_id.lock() = None;
        }

        to_js(&replace_plan.outcome)
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

    fn load_save_data_from_file(&mut self, name: &str, generation: u64) -> Result<JSValue> {
        let path = format!("saves/{}.sav", name);
        let runtime = self.runtime.clone();
        let backlog = self.backlog.clone();
        let checkpoints = self.checkpoints.clone();
        let current_marker_id = self.current_marker_id.clone();
        let execution_generation = self.execution_generation.clone();
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

            let GameData {
                current_state,
                records,
                blocks,
            } = data;

            if is_execution_stale(&execution_generation, generation) {
                log::info!("Skip stale load game result for {}", path);
                return Ok(());
            }

            ScenarioPlugin::restore_runtime_snapshot(&runtime, &current_state, &blocks)?;

            {
                let mut backlog = backlog.lock();
                backlog.records = records;
                backlog.blocks = blocks;
                backlog.next_record_serial = backlog.records.len() as u64;
            }

            checkpoints.lock().clear();
            *current_marker_id.lock() = None;

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
            let game_data = serde_json::to_vec_pretty(&self.build_game_data()?)?;

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
        let _ = zip.set_comment(ZIP_COMMENT);

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
        let execution_generation = self.execution_generation.clone();
        let generation = execution_generation.load(Ordering::Relaxed);
        let future = async move {
            let result = run_step_loop(&runtime, &execution_generation, generation).await;
            if !is_execution_stale(&execution_generation, generation) {
                disable_next_line.store(false, Ordering::Relaxed);
            }
            result
        };

        create_promise(future)
    }

    pub fn warp(&mut self, marker_id: &str, boundary: WarpBoundary) -> Result<JSValue> {
        let generation = self.begin_runtime_transition();
        self.disable_next_line.store(true, Ordering::Relaxed);

        *self.warp_state.lock() =
            Some(WarpState::new(marker_id.to_string(), boundary));

        let runtime = self.runtime.clone();
        let sender = self.sender.clone();
        let warp_state = self.warp_state.clone();
        let current_marker_id = self.current_marker_id.clone();
        let disable_next_line = self.disable_next_line.clone();
        let execution_generation = self.execution_generation.clone();
        let target_marker_id = marker_id.to_string();

        let future = async move {
            let outcome = run_warp_loop(
                &runtime,
                &execution_generation,
                generation,
                &warp_state,
            )
            .await;

            let replay_state = warp_state.lock().take();

            if !is_execution_stale(&execution_generation, generation) {
                disable_next_line.store(false, Ordering::Relaxed);
            }

            let outcome = outcome?;

            let Some(replay_state) = replay_state else {
                return Ok::<(), anyhow::Error>(());
            };

            if matches!(outcome, WarpLoopOutcome::Stale) {
                return Ok::<(), anyhow::Error>(());
            }

            if replay_state.reached_target {
                *current_marker_id.lock() = Some(target_marker_id);
            }

            for event in replay_state.events {
                sender.send(event).await.map_err(anyhow::Error::from)?;
            }

            if matches!(outcome, WarpLoopOutcome::ReachedTarget) {
                sender
                    .send(ScenarioEvent::WarpFinished)
                    .await
                    .map_err(anyhow::Error::from)?;
            }

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
    execution_generation: &Arc<AtomicU64>,
    generation: u64,
) -> std::result::Result<(), anyhow::Error> {
    use moyu_core::utils::eval_in_sandbox::eval_in_sandbox;
    use moyu_pal::dir::assets_dir;

    let mut steps: usize = 0;

    loop {
        if is_execution_stale(execution_generation, generation) {
            return Ok(());
        }

        steps += 1;
        if steps > MAX_STEPS_PER_RUN {
            return Err(anyhow::anyhow!(
                "Scenario execution exceeded the maximum step limit per run ({} steps). \
                 This is likely caused by an unconditional infinite loop in the script \
                 (e.g. a #goto/#replace cycle with no reachable exit).",
                MAX_STEPS_PER_RUN
            ));
        }

        let step_result = {
            let mut rt = runtime.lock();
            if is_execution_stale(execution_generation, generation) {
                return Ok(());
            }
            rt.step().map_err(anyhow::Error::from)
        };

        match step_result? {
            StepResult::Done => return Ok(()),
            StepResult::NeedsCondition(condition) => {
                let ret = eval_in_sandbox(format!("Boolean({})", condition)).await?;
                let result = ret.as_bool().unwrap_or(false);
                if is_execution_stale(execution_generation, generation) {
                    return Ok(());
                }

                let mut rt = runtime.lock();
                if is_execution_stale(execution_generation, generation) {
                    return Ok(());
                }
                rt.resume_condition(result);
            }
            StepResult::NeedsScript(script) => {
                let ret = eval_in_sandbox(format!("(() => {{ {} }})()", script)).await?;
                let literal = convert_to_literal(ret);
                if is_execution_stale(execution_generation, generation) {
                    return Ok(());
                }

                let mut rt = runtime.lock();
                if is_execution_stale(execution_generation, generation) {
                    return Ok(());
                }
                rt.resume_script(Some(RValue::Literal(literal)), true);
            }
            StepResult::NeedsStoryFile(story_name) => {
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
                if is_execution_stale(execution_generation, generation) {
                    return Ok(());
                }

                let mut rt = runtime.lock();
                if is_execution_stale(execution_generation, generation) {
                    return Ok(());
                }
                rt.provide_story_data(&story_name, data)
                    .map_err(anyhow::Error::from)?;
            }
        }
    }
}

/// Maximum number of outer iterations in run_warp_loop.
/// Each iteration corresponds to one complete run_step_loop call (ending at a
/// text line or command). This bounds the total number of "segments" traversed
/// during a single warp, preventing infinite loops caused by goto cycles that
/// always pass through at least one text/command line before looping back.
const MAX_WARP_OUTER_ITERATIONS: usize = 100000;

enum WarpLoopOutcome {
    ReachedTarget,
    FinishedWithoutTarget,
    Stale,
}

async fn run_warp_loop(
    runtime: &Arc<Mutex<Runtime<ScenarioExecutor>>>,
    execution_generation: &Arc<AtomicU64>,
    generation: u64,
    warp_state: &Arc<Mutex<Option<WarpState>>>,
) -> std::result::Result<WarpLoopOutcome, anyhow::Error> {
    let mut outer_iterations: usize = 0;

    loop {
        if is_execution_stale(execution_generation, generation) {
            return Ok(WarpLoopOutcome::Stale);
        }

        outer_iterations += 1;
        if outer_iterations > MAX_WARP_OUTER_ITERATIONS {
            return Err(anyhow::anyhow!(
                "Warp exceeded the maximum outer iteration limit ({} iterations). \
                 The target marker may be unreachable due to a goto/replace loop \
                 in the script.",
                MAX_WARP_OUTER_ITERATIONS
            ));
        }

        run_step_loop(runtime, execution_generation, generation).await?;

        if is_execution_stale(execution_generation, generation) {
            return Ok(WarpLoopOutcome::Stale);
        }

        let warp_state = warp_state.lock();
        let Some(state) = warp_state.as_ref() else {
            return Ok(WarpLoopOutcome::Stale);
        };

        if state.reached_target {
            return Ok(WarpLoopOutcome::ReachedTarget);
        }

        if state.reached_finished {
            return Ok(WarpLoopOutcome::FinishedWithoutTarget);
        }
    }
}

fn is_execution_stale(execution_generation: &Arc<AtomicU64>, generation: u64) -> bool {
    execution_generation.load(Ordering::Relaxed) != generation
}

impl Plugin for ScenarioPlugin {
    fn update(&mut self, _: bool) {
        if let Some(state) = self.waiting.as_ref() {
            if moyu_pal::time::Instant::now() >= state.until {
                let _ = self.waiting.take();

                let runtime = self.runtime.clone();
                let execution_generation = self.execution_generation.clone();
                let generation = execution_generation.load(Ordering::Relaxed);
                moyu_pal::task::spawn(async move {
                    if let Err(err) =
                        run_step_loop(&runtime, &execution_generation, generation).await
                    {
                        log::error!("Error during scenario execution after wait: {}", err);
                    }
                });
            }
        }

        let mut events = Vec::new();
        while let Some(event) = self.take_queued_event() {
            events.push(event);
        }

        for event in events {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MarkerEnter, TextLine};
    use sixu::format::Literal;
    use sixu::parser::parse;
    use sixu::runtime::StepResult;
    use std::sync::Once;

    static PLATFORM_SETUP: Once = Once::new();

    fn setup_test_platform() {
        PLATFORM_SETUP.call_once(|| {
            let _ = moyu_pal::platform::setup();
        });
    }

    fn build_plugin(script: &str) -> ScenarioPlugin {
        let (_, story) = parse("test", script).unwrap();
        let plugin = ScenarioPlugin::new();
        {
            let mut runtime = plugin.runtime.lock();
            runtime.add_story(story);
            runtime.start("test", Some("entry")).unwrap();
        }
        plugin
    }

    #[tokio::test]
    async fn checkpoint_roundtrip_restores_variables_and_cursor() {
        setup_test_platform();

        let mut plugin = build_plugin(
            r#"
::entry {
//#marker id=Lstart
text
after
}
"#,
        );

        {
            let mut runtime = plugin.runtime.lock();
            runtime
                .context_mut()
                .archive_variables_mut()
                .as_object_mut()
                .unwrap()
                .insert("value".to_string(), Literal::Integer(1));
            runtime
                .context_mut()
                .set_local("choice".to_string(), Literal::String("alpha".to_string()))
                .unwrap();
            assert!(matches!(runtime.step(), Ok(StepResult::Done)));
        }

        while let Ok(event) = plugin.receiver.try_recv() {
            plugin.track_cursor_event(&event);
        }

        plugin.capture_checkpoint_state("cp1").unwrap();

        {
            let mut runtime = plugin.runtime.lock();
            runtime
                .context_mut()
                .archive_variables_mut()
                .as_object_mut()
                .unwrap()
                .insert("value".to_string(), Literal::Integer(9));
            runtime
                .context_mut()
                .set_local("choice".to_string(), Literal::String("beta".to_string()))
                .unwrap();
        }
        *plugin.current_marker_id.lock() = Some("Lchanged".to_string());

        assert!(plugin.restore_checkpoint_state("cp1").unwrap());

        let runtime = plugin.runtime.lock();
        assert_eq!(
            runtime
                .context()
                .archive_variables()
                .as_object()
                .unwrap()
                .get("value"),
            Some(&Literal::Integer(1))
        );
        assert_eq!(
            runtime
                .context()
                .get_local("choice"),
            Some(&Literal::String("alpha".to_string()))
        );
        drop(runtime);

        let cursor = plugin.build_execution_cursor().unwrap().unwrap();
        assert_eq!(cursor.story, "test");
        assert_eq!(cursor.paragraph, "entry");
        assert_eq!(cursor.marker_id.as_deref(), Some("Lstart"));
    }

    #[tokio::test]
    async fn restore_checkpoint_clears_stale_events() {
        setup_test_platform();

        let mut plugin = build_plugin(
            r#"
::entry {
text
}
"#,
        );

        plugin.capture_checkpoint_state("cp1").unwrap();
        plugin
            .sender
            .try_send(ScenarioEvent::Text(TextLine {
                leading: None,
                text: Some("stale".to_string()),
                tailing: None,
            }))
            .unwrap();

        assert!(plugin.restore_checkpoint_state("cp1").unwrap());
        assert!(plugin.receiver.try_recv().is_err());
    }

    #[test]
    fn queued_events_are_drained_in_order() {
        let mut plugin = ScenarioPlugin::new();

        plugin
            .sender
            .try_send(ScenarioEvent::MarkerEnter(MarkerEnter {
                marker_id: "L1".to_string(),
                story: "test".to_string(),
                paragraph: "entry".to_string(),
            }))
            .unwrap();
        plugin
            .sender
            .try_send(ScenarioEvent::Text(TextLine {
                leading: None,
                text: Some("hello".to_string()),
                tailing: None,
            }))
            .unwrap();
        plugin.sender.try_send(ScenarioEvent::Finished).unwrap();

        let mut drained = Vec::new();
        while let Some(event) = plugin.take_queued_event() {
            drained.push(event);
        }

        assert_eq!(drained.len(), 3);
        assert!(matches!(drained[0], ScenarioEvent::MarkerEnter(_)));
        assert!(matches!(drained[1], ScenarioEvent::Text(_)));
        assert!(matches!(drained[2], ScenarioEvent::Finished));
        assert_eq!(*plugin.current_marker_id.lock(), None);
    }

    #[test]
    fn marked_text_emits_marker_after_line_event_and_stores_remote_checkpoint() {
        let mut plugin = build_plugin(
            r#"
::entry {
//#marker id=L1
text
}
"#,
        );

        {
            let mut runtime = plugin.runtime.lock();
            assert!(matches!(runtime.step(), Ok(StepResult::Done)));
        }

        let first = plugin.take_queued_event().unwrap();
        let second = plugin.take_queued_event().unwrap();

        assert!(matches!(first, ScenarioEvent::Text(_)));
        assert!(matches!(second, ScenarioEvent::MarkerEnter(_)));

        let checkpoint = plugin.checkpoints.lock().get("L1").cloned().unwrap();
        assert_eq!(checkpoint.cursor.unwrap().marker_id.as_deref(), Some("L1"));
    }

    #[test]
    fn fast_forward_checkpoint_uses_latest_previous_marker_in_current_story() {
        let plugin = build_plugin(
            r#"
::entry {
//#marker id=L1
first
//#marker id=L2
second
//#marker id=L3
third
}
"#,
        );

        {
            let mut runtime = plugin.runtime.lock();
            assert!(matches!(runtime.step(), Ok(StepResult::Done)));
            assert!(matches!(runtime.step(), Ok(StepResult::Done)));
        }

        assert_eq!(
            plugin.get_fast_forward_checkpoint_info("L3").unwrap(),
            (true, Some("L2".to_string()))
        );
    }

    #[test]
    fn fast_forward_checkpoint_reports_missing_previous_checkpoint_for_first_marker() {
        let plugin = build_plugin(
            r#"
::entry {
//#marker id=L1
first
//#marker id=L2
second
}
"#,
        );

        assert_eq!(
            plugin.get_fast_forward_checkpoint_info("L1").unwrap(),
            (true, None)
        );
    }
}
