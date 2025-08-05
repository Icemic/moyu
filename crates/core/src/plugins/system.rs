use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwapOption;
use moyu_pal::config::WindowState;
use serde::{Deserialize, Serialize};

use crate::base::Snapshot;
use crate::core::Core;
use crate::traits::{Command, Plugin};
use crate::utils::convert::{create_promise, from_js, to_js, JSValue};

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
pub enum SystemCommmad {
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
    TakeSnapshot,
    Quit,
}

impl Command for SystemPlugin {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let payload: SystemCommmad = from_js(payload)?;
        match payload {
            SystemCommmad::SetWindowSize {
                width,
                height,
                factor,
            } => {
                self.core.resize_window(width, height, factor);
                self.core.move_to_center();
            }
            SystemCommmad::SetWindowState { state } => {
                self.core.set_window_state(state);
            }
            SystemCommmad::SetTitle { title } => {
                self.core.window().set_title(&title);
            }
            SystemCommmad::GetWindowState => {
                let state = self.core.get_window_state();
                return Ok(Some(to_js(&state)?));
            }
            SystemCommmad::GetWindowInnerPosition => {
                let scale_factor = self.core.window().scale_factor();
                let position = self.core.window().inner_position()?;
                let position: winit::dpi::LogicalPosition<i32> = position.to_logical(scale_factor);
                return Ok(Some(to_js(&position)?));
            }
            SystemCommmad::GetWindowInnerSize => {
                let scale_factor = self.core.window().scale_factor();
                let size = self.core.window().inner_size();
                let size: winit::dpi::LogicalSize<u32> = size.to_logical(scale_factor);
                return Ok(Some(to_js(&size)?));
            }
            SystemCommmad::TakeSnapshot => {
                if let Some(graphics) = self.core.graphics() {
                    graphics.request_snapshot();

                    // Create an async function that will poll for the snapshot
                    let graphics_clone = graphics.clone();
                    let snapshot_store = self.snapshot.clone();
                    let fut = async move {
                        // Poll until the snapshot is ready
                        loop {
                            if let Some((data, width, height, format)) =
                                graphics_clone.try_get_snapshot()
                            {
                                let snapshot = Snapshot {
                                    width,
                                    height,
                                    data,
                                    format: format.into(),
                                };

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
            SystemCommmad::Quit => {
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
