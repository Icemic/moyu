//! Engine manifest, local metadata, and version state.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::DomainModel;
use crate::config::{DEFAULT_CDN_URL, DEFAULT_CHANNEL};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineConfig {
    pub cdn_url: String,
    pub version: Option<String>,
    pub channel: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            cdn_url: DEFAULT_CDN_URL.to_string(),
            version: None,
            channel: DEFAULT_CHANNEL.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionsJson {
    pub schema_version: u32,
    pub channels: BTreeMap<String, ChannelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub latest: String,
    pub versions: BTreeMap<String, VersionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    pub published_at: String,
    pub assets: BTreeMap<String, AssetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDownload {
    pub sha256: String,
    pub downloaded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedVersion {
    pub channel: String,
    pub published_at: String,
    pub platforms: BTreeMap<String, PlatformDownload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveEngine {
    pub version: String,
    pub channel: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineMeta {
    pub active: Option<ActiveEngine>,
    #[serde(default)]
    pub downloads: BTreeMap<String, DownloadedVersion>,
}

#[derive(Debug, Clone, Default)]
pub struct EngineModel {
    pub config: EngineConfig,
    pub meta: EngineMeta,
    pub remote_manifest: Option<VersionsJson>,
    pub fetching_manifest: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    ConfigLoaded(EngineConfig),
    MetaLoaded(EngineMeta),
    ManifestFetching,
    ManifestLoaded(VersionsJson),
    ActiveSwitched(ActiveEngine),
    Error(String),
}

impl DomainModel for EngineModel {
    type Event = EngineEvent;

    fn apply(&mut self, event: Self::Event) -> bool {
        match event {
            EngineEvent::ConfigLoaded(cfg) => {
                self.config = cfg;
                true
            }
            EngineEvent::MetaLoaded(meta) => {
                self.meta = meta;
                true
            }
            EngineEvent::ManifestFetching => {
                self.fetching_manifest = true;
                self.last_error = None;
                true
            }
            EngineEvent::ManifestLoaded(manifest) => {
                self.fetching_manifest = false;
                self.remote_manifest = Some(manifest);
                self.last_error = None;
                true
            }
            EngineEvent::ActiveSwitched(active) => {
                self.meta.active = Some(active);
                true
            }
            EngineEvent::Error(err) => {
                self.fetching_manifest = false;
                self.last_error = Some(err);
                true
            }
        }
    }
}
