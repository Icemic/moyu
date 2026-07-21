use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use moyu_core::traits::Command;
use moyu_core::utils::convert::{JSValue, from_js, to_js};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::SteamPlugin;

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DlcProgress {
    pub downloaded_bytes: String,
    pub total_bytes: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum OverlayDialog {
    Friends,
    Community,
    Players,
    Settings,
    OfficialGameGroup,
    Stats,
    Achievements,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum OverlayStoreFlag {
    None,
    AddToCart,
    AddToCartAndShow,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum OverlayNotificationPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TimelinePossibleClip {
    None,
    Standard,
    Featured,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "subCommand"
)]
#[ts(export, optional_fields)]
pub enum SteamCommand {
    AchievementSet {
        name: String,
    },
    AchievementClear {
        name: String,
    },
    AchievementClearAll,
    AchievementGet {
        name: String,
    },
    AchievementIndicateProgress {
        name: String,
        current: u32,
        max: u32,
    },
    AppsDlcInstalled {
        app_id: u32,
    },
    AppsDlcProgress {
        app_id: u32,
    },
    AppsGetAppBuildId,
    AppsGetCurrentBetaName,
    AppsGetCurrentGameLanguage,
    AppsGetSteamUiLanguage,
    AppsInstallDlc {
        app_id: u32,
    },
    AppsIsSubscribedApp {
        app_id: u32,
    },
    AppsUninstallDlc {
        app_id: u32,
    },
    OverlayActivate {
        dialog: OverlayDialog,
    },
    OverlayActivateToStore {
        app_id: u32,
        flag: Option<OverlayStoreFlag>,
    },
    OverlayActivateToWebPage {
        url: String,
    },
    OverlayIsEnabled,
    OverlayNeedsPresent,
    OverlaySetNotificationPosition {
        position: OverlayNotificationPosition,
    },
    StatsClearAchievement {
        name: String,
    },
    StatsGetAchievement {
        name: String,
    },
    StatsGetFloatStat {
        name: String,
    },
    StatsGetIntStat {
        name: String,
    },
    StatsSetAchievement {
        name: String,
    },
    StatsIndicateAchievementProgress {
        name: String,
        current: u32,
        max: u32,
    },
    StatsListAchievements,
    StatsSetFloatStat {
        name: String,
        value: f64,
    },
    StatsSetIntStat {
        name: String,
        value: i32,
    },
    StatsStoreStats,
    TimelineAddEvent {
        icon: String,
        title: String,
        description: String,
        priority: Option<u32>,
        start_offset_seconds: Option<f64>,
        duration_seconds: Option<f64>,
        possible_clip: Option<TimelinePossibleClip>,
    },
    TimelineClearStateDescription {
        time_delta_seconds: Option<f64>,
    },
    TimelineSetStateDescription {
        description: String,
        time_delta_seconds: Option<f64>,
    },
    UserGetAccountId,
    UserGetCSteamId,
    UserGetGameBadgeLevel {
        series: i32,
        foil: bool,
    },
    UserGetPersonaName,
    WorkshopGetSubscribedItemPath {
        item_id: String,
    },
    WorkshopGetSubscribedItems {
        include_disabled: Option<bool>,
    },
}

fn ensure_c_string(value: &str, field: &str) -> Result<()> {
    if value.contains('\0') {
        bail!("{field} contains an embedded null byte");
    }
    Ok(())
}

impl Command for SteamPlugin {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let command: SteamCommand = from_js(payload)?;

        let command = match command {
            SteamCommand::AchievementSet { name } => {
                ensure_c_string(&name, "name")?;
                self.client
                    .user_stats()
                    .achievement(&name)
                    .set()
                    .map_err(|()| anyhow!("Steam SetAchievement failed"))?;
                self.client
                    .user_stats()
                    .store_stats()
                    .map_err(|()| anyhow!("Steam StoreStats failed"))?;
                return Ok(None);
            }
            SteamCommand::AchievementClear { name } => {
                ensure_c_string(&name, "name")?;
                self.client
                    .user_stats()
                    .achievement(&name)
                    .clear()
                    .map_err(|()| anyhow!("Steam ClearAchievement failed"))?;
                self.client
                    .user_stats()
                    .store_stats()
                    .map_err(|()| anyhow!("Steam StoreStats failed"))?;
                return Ok(None);
            }
            SteamCommand::AchievementClearAll => {
                let names = self
                    .client
                    .user_stats()
                    .get_achievement_names()
                    .ok_or_else(|| anyhow!("Steam ListAchievements failed"))?;
                for name in names {
                    self.client
                        .user_stats()
                        .achievement(&name)
                        .clear()
                        .map_err(|()| anyhow!("Steam ClearAchievement failed"))?;
                }
                self.client
                    .user_stats()
                    .store_stats()
                    .map_err(|()| anyhow!("Steam StoreStats failed"))?;
                return Ok(None);
            }
            SteamCommand::AchievementGet { name } => {
                ensure_c_string(&name, "name")?;
                let achieved = self
                    .client
                    .user_stats()
                    .achievement(&name)
                    .get()
                    .map_err(|_| anyhow!("Steam GetAchievement failed for {name}"))?;
                return Ok(Some(to_js(&achieved)?));
            }
            SteamCommand::AchievementIndicateProgress { name, current, max } => {
                ensure_c_string(&name, "name")?;
                if max == 0 {
                    bail!("max must be greater than 0");
                }
                if current >= max {
                    bail!("current must be less than max");
                }
                self.client
                    .user_stats()
                    .indicate_achievement_progress(&name, current, max)?;
                return Ok(None);
            }
            command => command,
        };

        let client = &self.client;
        match command {
            SteamCommand::AppsDlcInstalled { app_id } => Ok(Some(to_js(
                &client.apps().is_dlc_installed(steamworks::AppId(app_id)),
            )?)),
            SteamCommand::AppsDlcProgress { app_id } => {
                let progress = client
                    .apps()
                    .dlc_download_progress(steamworks::AppId(app_id))
                    .map(|progress| DlcProgress {
                        downloaded_bytes: progress.downloaded_bytes.to_string(),
                        total_bytes: progress.total_bytes.to_string(),
                    });
                Ok(Some(to_js(&progress)?))
            }
            SteamCommand::AppsGetAppBuildId => Ok(Some(to_js(&client.apps().app_build_id())?)),
            SteamCommand::AppsGetCurrentBetaName => {
                Ok(Some(to_js(&client.apps().current_beta_name())?))
            }
            SteamCommand::AppsGetCurrentGameLanguage => {
                Ok(Some(to_js(&client.apps().current_game_language())?))
            }
            SteamCommand::AppsGetSteamUiLanguage => Ok(Some(to_js(&client.utils().ui_language())?)),
            SteamCommand::AppsInstallDlc { app_id } => {
                client.apps().install_dlc(steamworks::AppId(app_id));
                Ok(None)
            }
            SteamCommand::AppsIsSubscribedApp { app_id } => Ok(Some(to_js(
                &client.apps().is_subscribed_app(steamworks::AppId(app_id)),
            )?)),
            SteamCommand::AppsUninstallDlc { app_id } => {
                client.apps().uninstall_dlc(steamworks::AppId(app_id));
                Ok(None)
            }
            SteamCommand::OverlayActivate { dialog } => {
                let dialog = match dialog {
                    OverlayDialog::Friends => "Friends",
                    OverlayDialog::Community => "Community",
                    OverlayDialog::Players => "Players",
                    OverlayDialog::Settings => "Settings",
                    OverlayDialog::OfficialGameGroup => "OfficialGameGroup",
                    OverlayDialog::Stats => "Stats",
                    OverlayDialog::Achievements => "Achievements",
                };
                client.friends().activate_game_overlay(dialog)?;
                Ok(None)
            }
            SteamCommand::OverlayActivateToStore { app_id, flag } => {
                let flag = match flag.unwrap_or(OverlayStoreFlag::None) {
                    OverlayStoreFlag::None => steamworks::OverlayToStoreFlag::None,
                    OverlayStoreFlag::AddToCart => steamworks::OverlayToStoreFlag::AddToCart,
                    OverlayStoreFlag::AddToCartAndShow => {
                        steamworks::OverlayToStoreFlag::AddToCartAndShow
                    }
                };
                client
                    .friends()
                    .activate_game_overlay_to_store(steamworks::AppId(app_id), flag);
                Ok(None)
            }
            SteamCommand::OverlayActivateToWebPage { url } => {
                ensure_c_string(&url, "url")?;
                client.friends().activate_game_overlay_to_web_page(&url)?;
                Ok(None)
            }
            SteamCommand::OverlayIsEnabled => {
                Ok(Some(to_js(&client.utils().is_overlay_enabled())?))
            }
            SteamCommand::OverlayNeedsPresent => {
                Ok(Some(to_js(&client.utils().overlay_needs_present())?))
            }
            SteamCommand::OverlaySetNotificationPosition { position } => {
                let position = match position {
                    OverlayNotificationPosition::TopLeft => {
                        steamworks::NotificationPosition::TopLeft
                    }
                    OverlayNotificationPosition::TopRight => {
                        steamworks::NotificationPosition::TopRight
                    }
                    OverlayNotificationPosition::BottomLeft => {
                        steamworks::NotificationPosition::BottomLeft
                    }
                    OverlayNotificationPosition::BottomRight => {
                        steamworks::NotificationPosition::BottomRight
                    }
                };
                client.utils().set_overlay_notification_position(position);
                Ok(None)
            }
            SteamCommand::StatsClearAchievement { name } => {
                ensure_c_string(&name, "name")?;
                client
                    .user_stats()
                    .achievement(&name)
                    .clear()
                    .map_err(|()| anyhow!("Steam ClearAchievement failed"))?;
                Ok(None)
            }
            SteamCommand::StatsGetAchievement { name } => {
                ensure_c_string(&name, "name")?;
                let value = client.user_stats().achievement(&name).get().ok();
                Ok(Some(to_js(&value)?))
            }
            SteamCommand::StatsGetFloatStat { name } => {
                ensure_c_string(&name, "name")?;
                let value = client.user_stats().get_stat_f32(&name).ok();
                Ok(Some(to_js(&value)?))
            }
            SteamCommand::StatsGetIntStat { name } => {
                ensure_c_string(&name, "name")?;
                let value = client.user_stats().get_stat_i32(&name).ok();
                Ok(Some(to_js(&value)?))
            }
            SteamCommand::StatsSetAchievement { name } => {
                ensure_c_string(&name, "name")?;
                client
                    .user_stats()
                    .achievement(&name)
                    .set()
                    .map_err(|()| anyhow!("Steam SetAchievement failed"))?;
                Ok(None)
            }
            SteamCommand::StatsIndicateAchievementProgress { name, current, max } => {
                ensure_c_string(&name, "name")?;
                if max == 0 {
                    bail!("max must be greater than 0");
                }
                if current >= max {
                    bail!("current must be less than max");
                }
                client
                    .user_stats()
                    .indicate_achievement_progress(&name, current, max)?;
                Ok(None)
            }
            SteamCommand::StatsListAchievements => {
                let names = client
                    .user_stats()
                    .get_achievement_names()
                    .ok_or_else(|| anyhow!("Steam ListAchievements failed"))?;
                Ok(Some(to_js(&names)?))
            }
            SteamCommand::StatsSetFloatStat { name, value } => {
                ensure_c_string(&name, "name")?;
                client
                    .user_stats()
                    .set_stat_f32(&name, value as f32)
                    .map_err(|()| anyhow!("Steam SetStat failed"))?;
                Ok(None)
            }
            SteamCommand::StatsSetIntStat { name, value } => {
                ensure_c_string(&name, "name")?;
                client
                    .user_stats()
                    .set_stat_i32(&name, value)
                    .map_err(|()| anyhow!("Steam SetStat failed"))?;
                Ok(None)
            }
            SteamCommand::StatsStoreStats => {
                client
                    .user_stats()
                    .store_stats()
                    .map_err(|()| anyhow!("Steam StoreStats failed"))?;
                Ok(None)
            }
            SteamCommand::TimelineAddEvent {
                icon,
                title,
                description,
                priority,
                start_offset_seconds,
                duration_seconds,
                possible_clip,
            } => {
                let priority = priority.unwrap_or(0);
                if priority > 1000 {
                    bail!("priority must be between 0 and 1000");
                }
                let start_offset_seconds = start_offset_seconds.unwrap_or_default() as f32;
                let duration = Duration::try_from_secs_f64(duration_seconds.unwrap_or_default())
                    .map_err(|_| anyhow!("durationSeconds must be a non-negative finite number"))?;
                let possible_clip = match possible_clip.unwrap_or(TimelinePossibleClip::None) {
                    TimelinePossibleClip::None => steamworks::TimelineEventClipPriority::None,
                    TimelinePossibleClip::Standard => {
                        steamworks::TimelineEventClipPriority::Standard
                    }
                    TimelinePossibleClip::Featured => {
                        steamworks::TimelineEventClipPriority::Featured
                    }
                };
                client.timeline().add_timeline_event(
                    &icon,
                    &title,
                    &description,
                    priority,
                    start_offset_seconds,
                    duration,
                    possible_clip,
                )?;
                Ok(None)
            }
            SteamCommand::TimelineClearStateDescription { time_delta_seconds } => {
                let duration = Duration::try_from_secs_f64(time_delta_seconds.unwrap_or_default())
                    .map_err(|_| {
                        anyhow!("timeDeltaSeconds must be a non-negative finite number")
                    })?;
                client
                    .timeline()
                    .clear_timeline_state_description(duration)?;
                Ok(None)
            }
            SteamCommand::TimelineSetStateDescription {
                description,
                time_delta_seconds,
            } => {
                let duration = Duration::try_from_secs_f64(time_delta_seconds.unwrap_or_default())
                    .map_err(|_| {
                        anyhow!("timeDeltaSeconds must be a non-negative finite number")
                    })?;
                client
                    .timeline()
                    .set_timeline_state_description(&description, duration)?;
                Ok(None)
            }
            SteamCommand::UserGetAccountId => {
                Ok(Some(to_js(&client.user().steam_id().account_id().raw())?))
            }
            SteamCommand::UserGetCSteamId => {
                Ok(Some(to_js(&client.user().steam_id().raw().to_string())?))
            }
            SteamCommand::UserGetGameBadgeLevel { series, foil } => {
                Ok(Some(to_js(&client.user().game_badge_level(series, foil))?))
            }
            SteamCommand::UserGetPersonaName => Ok(Some(to_js(&client.friends().name())?)),
            SteamCommand::WorkshopGetSubscribedItemPath { item_id } => {
                let item_id = steamworks::PublishedFileId(
                    item_id
                        .parse()
                        .map_err(|_| anyhow!("Invalid Workshop item ID: {item_id}"))?,
                );
                let path = client
                    .ugc()
                    .item_install_info(item_id)
                    .map(|info| info.folder);
                Ok(Some(to_js(&path)?))
            }
            SteamCommand::WorkshopGetSubscribedItems { include_disabled } => {
                let items = client
                    .ugc()
                    .subscribed_items(include_disabled.unwrap_or(false))
                    .into_iter()
                    .map(|item| item.0.to_string())
                    .collect::<Vec<_>>();
                Ok(Some(to_js(&items)?))
            }
            _ => unreachable!("achievement commands are handled before Steam client access"),
        }
    }
}
