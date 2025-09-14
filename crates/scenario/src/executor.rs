use moyu_pal::sync::mpsc::Sender;
use sixu::format::*;
use sixu::runtime::*;

use crate::types::ScenarioEvent;

/// Executor that implements the runtime execution logic for ScenarioPlugin
pub struct ScenarioExecutor {
    sender: Sender<ScenarioEvent>,
}

impl ScenarioExecutor {
    pub fn new(sender: Sender<ScenarioEvent>) -> Self {
        Self { sender }
    }
}

impl RuntimeExecutor for ScenarioExecutor {
    fn handle_command(
        &mut self,
        _ctx: &mut RuntimeContext,
        command_line: &CommandLine,
    ) -> sixu::error::Result<()> {
        self.sender
            .try_send(ScenarioEvent::CommandLine(command_line.clone()))
            .map_err(anyhow::Error::from)?;
        Ok(())
    }

    fn handle_extra_system_call(
        &mut self,
        _ctx: &mut RuntimeContext,
        systemcall_line: &SystemCallLine,
    ) -> sixu::error::Result<()> {
        self.sender
            .try_send(ScenarioEvent::ExtraSystemCall(systemcall_line.clone()))
            .map_err(anyhow::Error::from)?;
        Ok(())
    }

    fn handle_text(
        &mut self,
        _ctx: &mut RuntimeContext,
        leading: Option<&str>,
        text: Option<&str>,
    ) -> sixu::error::Result<()> {
        self.sender
            .try_send(ScenarioEvent::Text {
                leading: leading.map(|s| s.to_string()),
                text: text.map(|s| s.to_string()),
            })
            .map_err(anyhow::Error::from)?;
        Ok(())
    }

    fn eval_script(
        &mut self,
        _ctx: &mut RuntimeContext,
        _script: &String,
    ) -> sixu::error::Result<Option<RValue>> {
        // TODO: Implement actual script evaluation logic here
        // For now, return None
        log::warn!("Script evaluation not implemented");
        Ok(None)
    }

    fn finished(&mut self, _ctx: &mut RuntimeContext) {
        let _ = self.sender.send(ScenarioEvent::Finished);
    }
}
