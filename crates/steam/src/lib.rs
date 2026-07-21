//! Steam integration for Moyu desktop builds.
#![cfg(desktop)]

mod commands;
use moyu_core::traits::{Command, Plugin, PluginBaseTrait};
use moyu_macros::Plugin;

/// Checks whether the current process should exit so Steam can restart the app.
pub fn restart_app_if_necessary(app_id: u32) -> Result<bool, ()> {
    if !steamworks::steam_api_exists() {
        return Err(());
    }

    Ok(steamworks::restart_app_if_necessary(steamworks::AppId(
        app_id,
    )))
}

#[derive(Plugin)]
pub struct SteamPlugin {
    client: steamworks::Client,
}

impl SteamPlugin {
    pub fn new(app_id: u32) -> anyhow::Result<Self> {
        if !steamworks::steam_api_exists() {
            anyhow::bail!("Steam API dynamic library could not be loaded");
        }

        Ok(Self {
            client: steamworks::Client::init_app(app_id)?,
        })
    }
}

impl Plugin for SteamPlugin {
    fn plugin_name(&self) -> &'static str {
        "steam"
    }

    fn update(&mut self, _vsync: bool) {
        self.client.run_callbacks();
    }

    fn as_command(&mut self) -> Option<&mut dyn Command> {
        Some(self)
    }
}
