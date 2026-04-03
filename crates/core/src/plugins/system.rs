use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwapOption;
use moyu_macros::Plugin;
use moyu_pal::config::{WindowState, get_engine_config};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::base::Snapshot;
use crate::core::Core;
use crate::traits::PluginBaseTrait;
use crate::traits::{Command, Plugin};
use crate::utils::convert::{JSValue, create_promise, from_js, to_js};

#[derive(Plugin)]
pub struct SystemPlugin {
    core: Arc<Core>,
    snapshot: Arc<ArcSwapOption<Snapshot>>,
}

impl SystemPlugin {
    pub fn new(core: Arc<Core>) -> Self {
        Self {
            core,
            snapshot: Arc::new(ArcSwapOption::from(None)),
        }
    }

    pub fn snapshot(&self) -> Arc<ArcSwapOption<Snapshot>> {
        self.snapshot.clone()
    }
}

impl Plugin for SystemPlugin {
    fn plugin_name(&self) -> &'static str {
        "system"
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
#[derive(TS)]
#[ts(export, optional_fields)]
pub enum SystemCommand {
    SetWindowSize {
        width: f64,
        height: f64,
        factor: Option<f64>,
    },
    SetWindowState {
        state: WindowState,
    },
    SetTitle {
        title: String,
    },
    GetWindowState,
    GetWindowInnerPosition,
    GetWindowInnerSize,
    GetStageSize,
    TakeSnapshot {
        width: Option<u32>,
        height: Option<u32>,
        keep_aspect_ratio: Option<bool>,
    },
    GetParams,
    Quit,
}

impl Command for SystemPlugin {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let payload: SystemCommand = from_js(payload)?;
        match payload {
            SystemCommand::SetWindowSize {
                width,
                height,
                factor,
            } => {
                self.core.resize_window(width, height, factor);
                self.core.move_to_center();
            }
            SystemCommand::SetWindowState { state } => {
                self.core.set_window_state(state);
            }
            SystemCommand::SetTitle { title } => {
                self.core.window().set_title(&title);
            }
            SystemCommand::GetWindowState => {
                let state = self.core.get_window_state();
                return Ok(Some(to_js(&state)?));
            }
            SystemCommand::GetWindowInnerPosition => {
                let scale_factor = self.core.window().scale_factor();
                let position = self.core.window().inner_position()?;
                let position: winit::dpi::LogicalPosition<i32> = position.to_logical(scale_factor);
                return Ok(Some(to_js(&position)?));
            }
            SystemCommand::GetWindowInnerSize => {
                let scale_factor = self.core.window().scale_factor();
                let size = self.core.window().inner_size();
                let size: winit::dpi::LogicalSize<u32> = size.to_logical(scale_factor);
                return Ok(Some(to_js(&size)?));
            }
            SystemCommand::GetStageSize => {
                let size = self.core.stage_size();
                return Ok(Some(to_js(&size)?));
            }
            SystemCommand::TakeSnapshot {
                width,
                height,
                keep_aspect_ratio,
            } => {
                if let Some(graphics) = self.core.graphics() {
                    graphics.request_snapshot();

                    // Create an async function that will poll for the snapshot
                    let graphics_clone = graphics.clone();
                    let snapshot_store = self.snapshot.clone();
                    let fut = async move {
                        // Poll until the snapshot is ready
                        loop {
                            if let Some((
                                data,
                                origin_width,
                                origin_height,
                                bytes_per_row,
                                format,
                            )) = graphics_clone.try_get_snapshot()
                            {
                                let mut snapshot = Snapshot {
                                    width: origin_width,
                                    height: origin_height,
                                    data,
                                    stride: bytes_per_row,
                                    format: format.into(),
                                };

                                snapshot.resize(
                                    width.unwrap_or(u32::MAX),
                                    height.unwrap_or(u32::MAX),
                                    keep_aspect_ratio.unwrap_or(true),
                                )?;

                                snapshot_store.store(Some(Arc::new(snapshot)));
                                return Ok(());
                            }
                            // Small delay to avoid busy waiting
                            moyu_pal::time::sleep(std::time::Duration::from_millis(10)).await;
                        }
                    };

                    let promise = create_promise(fut)?;
                    return Ok(Some(promise));
                }
            }
            SystemCommand::GetParams => {
                return Ok(Some(to_js(&get_engine_config().params)?));
            }
            SystemCommand::Quit => {
                self.core.quit();
            }
        }

        Ok(None)
    }
}

// impl PluginEventSource for SystemPlugin {
//     type Event = SystemEvent;
// }

// #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub enum SystemEvent {
//     WindowState,
// }

// impl Event for SystemEvent {
//     fn name(&self) -> &'static str {
//         "systemevent"
//     }
// }
