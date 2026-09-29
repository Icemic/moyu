//! Engine facts exposed through `engine:state`.

use moyu_core::base::SurfaceSize;
use moyu_core::core::try_get_core;
use moyu_pal::config::get_engine_config;
use serde::Serialize;

use super::DebugSession;
use crate::logs::{self, LogStats};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStateSnapshot {
    /// Entry file the engine was started with.
    pub entry: String,
    pub platform: &'static str,
    pub engine_version: &'static str,
    pub surface_size: SurfaceSize,
    pub node_count: usize,
    pub uptime_ms: u64,
    /// Whether the engine has finished starting up.
    pub ready: bool,
    pub logs: LogStats,
}

impl EngineStateSnapshot {
    /// `None` while the engine core does not exist yet.
    pub fn capture(session: &DebugSession) -> Option<Self> {
        let core = try_get_core()?;

        Some(Self {
            entry: get_engine_config().entry.clone().unwrap_or_default(),
            platform: platform_name(),
            engine_version: env!("CARGO_PKG_VERSION"),
            surface_size: core.surface_size(),
            node_count: core.node_map().len(),
            uptime_ms: session.started_at.elapsed().as_millis() as u64,
            ready: super::is_ready(),
            logs: logs::stats(),
        })
    }
}

pub(super) const fn platform_name() -> &'static str {
    #[cfg(windows)]
    {
        "windows"
    }
    #[cfg(macos)]
    {
        "macos"
    }
    #[cfg(linux)]
    {
        "linux"
    }
    #[cfg(android)]
    {
        "android"
    }
    #[cfg(ios)]
    {
        "ios"
    }
    #[cfg(web)]
    {
        "web"
    }
}
