use std::collections::HashMap;
use std::sync::Arc;

use moyu_pal::sync::Mutex;
use moyu_pal::sync::mpsc::Sender;
use sixu::Fingerprint;
use sixu::format::*;
use sixu::runtime::*;

use crate::types::{
    ExecutionCursor, MarkerEnter, RuntimeCheckpoint, ScenarioEvent, WarpBoundary, TextLine,
};
use crate::utils::create_runtime_snapshot_from_context;

#[derive(Debug)]
pub struct WarpState {
    pub target_marker_id: String,
    pub boundary: WarpBoundary,
    pub reached_target: bool,
    pub reached_finished: bool,
    pub events: Vec<ScenarioEvent>,
}

impl WarpState {
    pub fn new(target_marker_id: String, boundary: WarpBoundary) -> Self {
        Self {
            target_marker_id,
            boundary,
            reached_target: false,
            reached_finished: false,
            events: Vec::new(),
        }
    }

    pub fn record_event(&mut self, event: ScenarioEvent) {
        match &event {
            ScenarioEvent::MarkerEnter(marker) if marker.marker_id == self.target_marker_id => {
                self.reached_target = true;
                if matches!(self.boundary, WarpBoundary::After) {
                    self.events.push(event);
                }
            }
            ScenarioEvent::Finished => {
                self.reached_finished = true;
                if !self.reached_target {
                    self.events.push(event);
                }
            }
            _ => {
                if !self.reached_target {
                    self.events.push(event);
                }
            }
        }
    }
}

/// Executor that implements the runtime execution logic for ScenarioPlugin
pub struct ScenarioExecutor {
    sender: Sender<ScenarioEvent>,
    checkpoints: Arc<Mutex<HashMap<String, RuntimeCheckpoint>>>,
    checkpoint_blocks: Arc<Mutex<HashMap<Fingerprint, Block>>>,
    warp_state: Arc<Mutex<Option<WarpState>>>,
}

impl ScenarioExecutor {
    pub fn new(
        sender: Sender<ScenarioEvent>,
        checkpoints: Arc<Mutex<HashMap<String, RuntimeCheckpoint>>>,
        checkpoint_blocks: Arc<Mutex<HashMap<Fingerprint, Block>>>,
        warp_state: Arc<Mutex<Option<WarpState>>>,
    ) -> Self {
        Self {
            sender,
            checkpoints,
            checkpoint_blocks,
            warp_state,
        }
    }

    fn emit_event(&mut self, event: ScenarioEvent) -> sixu::error::Result<()> {
        let mut warp_state = self.warp_state.lock();
        if let Some(state) = warp_state.as_mut() {
            state.record_event(event);
            return Ok(());
        }

        drop(warp_state);

        self.sender.try_send(event).map_err(anyhow::Error::from)?;

        Ok(())
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

        let snapshot = {
            let mut checkpoint_blocks = self.checkpoint_blocks.lock();
            create_runtime_snapshot_from_context(ctx, &mut checkpoint_blocks)?
        };

        self.checkpoints.lock().insert(
            marker.id.clone(),
            RuntimeCheckpoint {
                cursor: Some(cursor.clone()),
                snapshot,
            },
        );

        self.emit_event(ScenarioEvent::MarkerEnter(MarkerEnter {
            marker_id: marker.id.clone(),
            story: cursor.story,
            paragraph: cursor.paragraph,
        }))?;

        Ok(())
    }

    fn handle_command(
        &mut self,
        _ctx: &mut RuntimeContext,
        command_line: &ResolvedCommandLine,
    ) -> sixu::error::Result<bool> {
        self.emit_event(ScenarioEvent::CommandLine(command_line.clone()))?;
        Ok(false)
    }

    fn handle_extra_system_call(
        &mut self,
        _ctx: &mut RuntimeContext,
        systemcall_line: &ResolvedSystemCallLine,
    ) -> sixu::error::Result<bool> {
        self.emit_event(ScenarioEvent::ExtraSystemCall(systemcall_line.clone()))?;
        Ok(false)
    }

    fn handle_text(
        &mut self,
        _ctx: &mut RuntimeContext,
        leading: Option<&str>,
        text: Option<&str>,
        tailing: Option<&str>,
    ) -> sixu::error::Result<bool> {
        self.emit_event(ScenarioEvent::Text(TextLine {
            leading: leading.map(|s| s.to_string()),
            text: text.map(|s| s.to_string()),
            tailing: tailing.map(|s| s.to_string()),
        }))?;
        Ok(false)
    }

    fn finished(&mut self, _ctx: &mut RuntimeContext) {
        let _ = self.emit_event(ScenarioEvent::Finished);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker_event(marker_id: &str) -> ScenarioEvent {
        ScenarioEvent::MarkerEnter(MarkerEnter {
            marker_id: marker_id.to_string(),
            story: "test".to_string(),
            paragraph: "entry".to_string(),
        })
    }

    #[test]
    fn warp_state_excludes_target_marker_for_before_boundary() {
        let mut state = WarpState::new("L2".to_string(), WarpBoundary::Before);

        state.record_event(marker_event("L1"));
        state.record_event(marker_event("L2"));

        assert!(state.reached_target);
        assert_eq!(state.events.len(), 1);
    }

    #[test]
    fn warp_state_keeps_target_marker_for_after_boundary() {
        let mut state = WarpState::new("L2".to_string(), WarpBoundary::After);

        state.record_event(marker_event("L1"));
        state.record_event(marker_event("L2"));

        assert!(state.reached_target);
        assert_eq!(state.events.len(), 2);
    }
}
