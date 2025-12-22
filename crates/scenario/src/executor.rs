use moyu_pal::dir::assets_dir;
use moyu_pal::sync::mpsc::Sender;
use sixu::format::*;
use sixu::runtime::*;

use crate::types::ScenarioEvent;
use crate::types::TextLine;

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

    fn eval_script(
        &mut self,
        _ctx: &mut RuntimeContext,
        _script: &String,
    ) -> sixu::error::Result<(Option<RValue>, bool)> {
        // TODO: Implement actual script evaluation logic here
        // For now, return None
        log::warn!("Script evaluation not implemented");
        Ok((None, true))
    }

    fn finished(&mut self, _ctx: &mut RuntimeContext) {
        let _ = self.sender.send(ScenarioEvent::Finished);
    }

    async fn read_story_file(
        &mut self,
        _ctx: &mut RuntimeContext,
        story_name: &str,
    ) -> sixu::error::Result<Vec<u8>> {
        let asset_full_path = assets_dir()
            .join(&format!("scenario/{}.sixu", story_name))
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to construct asset path for scenario {}: {}",
                    story_name,
                    e
                )
            })?;

        let data = match moyu_pal::fs::read(&asset_full_path).await {
            Ok(data) => data,
            Err(e) => {
                log::error!(
                    "Failed to read scenario file: {}, scenario loading may not work.",
                    e
                );
                return Err(e.into());
            }
        };

        log::info!("Loaded scenario from file: {}", asset_full_path);

        Ok(data)
    }
}
