//! Project manifest and multi-resolution asset inspection state.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::DomainModel;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MultiResMode {
    Off,
    Auto,
    Forced(f64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantFile {
    pub path: String,
    pub scale: f64,
    pub base: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MultiResReport {
    pub mode: Option<MultiResMode>,
    pub unused_by_scale: Vec<(f64, Vec<String>)>,
    pub all_unused_when_off: usize,
    pub undeclared_scales: Vec<f64>,
    pub missing_base: Vec<(String, String)>,
}

impl MultiResReport {
    pub fn is_clean(&self) -> bool {
        self.all_unused_when_off == 0
            && self.unused_by_scale.is_empty()
            && self.undeclared_scales.is_empty()
            && self.missing_base.is_empty()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIndexSummary {
    pub window_title: Option<String>,
    pub entry_filename: Option<String>,
    pub initial_surface_size: Option<String>,
    pub stage_size: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoyuConfigSummary {
    pub engine_channel: Option<String>,
    pub engine_cdn_url: Option<String>,
    pub android_app_name: Option<String>,
    pub android_application_id: Option<String>,
    pub android_version_name: Option<String>,
    pub android_version_code: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkSummary {
    pub name: Option<String>,
    pub title: Option<String>,
    pub version: Option<String>,
    pub constructible: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectModel {
    pub root: Option<PathBuf>,
    pub index_summary: Option<ProjectIndexSummary>,
    pub moyu_summary: Option<MoyuConfigSummary>,
    pub framework_summary: Option<FrameworkSummary>,
    pub multi_res_report: Option<MultiResReport>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ProjectEvent {
    Opened {
        root: PathBuf,
        index: ProjectIndexSummary,
        moyu: Option<MoyuConfigSummary>,
        framework: Option<FrameworkSummary>,
    },
    MultiResChecked(MultiResReport),
    Error(String),
    Cleared,
}

impl DomainModel for ProjectModel {
    type Event = ProjectEvent;

    fn apply(&mut self, event: Self::Event) -> bool {
        match event {
            ProjectEvent::Opened {
                root,
                index,
                moyu,
                framework,
            } => {
                self.root = Some(root);
                self.index_summary = Some(index);
                self.moyu_summary = moyu;
                self.framework_summary = framework;
                self.last_error = None;
                true
            }
            ProjectEvent::MultiResChecked(report) => {
                self.multi_res_report = Some(report);
                true
            }
            ProjectEvent::Error(err) => {
                self.last_error = Some(err);
                true
            }
            ProjectEvent::Cleared => {
                *self = Self::default();
                true
            }
        }
    }
}
