use std::collections::HashMap;
use std::sync::Arc;

use moyu_pal::sync::Mutex;
use moyu_pal::sync::mpsc::Sender;
use sixu::format::*;
use sixu::runtime::*;

use crate::types::{ExecutionCursor, MarkerEnter, RuntimeCheckpoint, ScenarioEvent, TextLine};
use crate::utils::create_runtime_snapshot_from_context;

/// Executor that implements the runtime execution logic for ScenarioPlugin
pub struct ScenarioExecutor {
    sender: Sender<ScenarioEvent>,
    checkpoints: Arc<Mutex<HashMap<String, RuntimeCheckpoint>>>,
}

impl ScenarioExecutor {
    pub fn new(
        sender: Sender<ScenarioEvent>,
        checkpoints: Arc<Mutex<HashMap<String, RuntimeCheckpoint>>>,
    ) -> Self {
        Self { sender, checkpoints }
    }
}

impl RuntimeExecutor for ScenarioExecutor {
    fn handle_marker(
        &mut self,
        ctx: &mut RuntimeContext,
        marker: &LineMarker,
    ) -> sixu::error::Result<()> {
        let current_state = ctx
            .stack()
            .last()
            .ok_or(sixu::error::RuntimeError::StoryNotStarted)?;

        let cursor = ExecutionCursor {
            story: current_state.story.clone(),
            paragraph: current_state.paragraph.clone(),
            marker_id: Some(marker.id.clone()),
        };

        let (snapshot, blocks) = create_runtime_snapshot_from_context(ctx)?;
        self.checkpoints.lock().insert(
            marker.id.clone(),
            RuntimeCheckpoint {
                cursor: Some(cursor.clone()),
                snapshot,
                blocks,
            },
        );

        self.sender
            .try_send(ScenarioEvent::MarkerEnter(MarkerEnter {
                marker_id: marker.id.clone(),
                story: cursor.story,
                paragraph: cursor.paragraph,
            }))
            .map_err(anyhow::Error::from)?;

        Ok(())
    }

    fn handle_command(
        &mut self,
        _ctx: &mut RuntimeContext,
        command_line: &ResolvedCommandLine,
    ) -> sixu::error::Result<bool> {
        self.sender
            .try_send(ScenarioEvent::CommandLine(command_line.clone()))
            .map_err(anyhow::Error::from)?;
        Ok(false)
    }

    fn handle_extra_system_call(
        &mut self,
        _ctx: &mut RuntimeContext,
        systemcall_line: &ResolvedSystemCallLine,
    ) -> sixu::error::Result<bool> {
        self.sender
            .try_send(ScenarioEvent::ExtraSystemCall(systemcall_line.clone()))
            .map_err(anyhow::Error::from)?;
        Ok(false)
    }

    fn handle_text(
        &mut self,
        _ctx: &mut RuntimeContext,
        leading: Option<&str>,
        text: Option<&str>,
        tailing: Option<&str>,
    ) -> sixu::error::Result<bool> {
        self.sender
            .try_send(ScenarioEvent::Text(TextLine {
                leading: leading.map(|s| s.to_string()),
                text: text.map(|s| s.to_string()),
                tailing: tailing.map(|s| s.to_string()),
            }))
            .map_err(anyhow::Error::from)?;
        Ok(false)
    }

    fn finished(&mut self, _ctx: &mut RuntimeContext) {
        let _ = self.sender.send(ScenarioEvent::Finished);
    }
}
