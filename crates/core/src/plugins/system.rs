use std::sync::Arc;

use anyhow::Result;
use doufu_pal::config::WindowState;
use serde::{Deserialize, Serialize};

use crate::core::Core;
use crate::traits::{Command, Plugin};
use crate::user_event::UserEvent;
use crate::utils::convert::{from_js, to_js, JSValue};

pub struct SystemPlugin {
    core: Arc<Core>,
}

impl SystemPlugin {
    pub fn new(core: Arc<Core>) -> Self {
        Self { core }
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
                // self.core.resize_window(width, height, factor);

                // Still have to use UserEvent due to `.resize_stage` must be called outside a render pass
                self.core
                    .send_event(UserEvent::ResizeWindow(width, height, factor));
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
            SystemCommmad::Quit => {
                self.core.send_event(UserEvent::Quit);
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
