//! Node.js and package manager environment state model.

use std::path::PathBuf;

use super::DomainModel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Yarn,
    Pnpm,
    Npm,
}

impl PackageManager {
    pub fn binary_name(self) -> &'static str {
        match self {
            Self::Yarn => "yarn",
            Self::Pnpm => "pnpm",
            Self::Npm => "npm",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRuntimeInfo {
    pub executable: PathBuf,
    pub version: String,
    pub major: u32,
    pub is_managed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EnvModel {
    pub node: Option<NodeRuntimeInfo>,
    pub package_manager: Option<PackageManager>,
    pub available_package_managers: Vec<PackageManager>,
    pub jiti_path: Option<PathBuf>,
    pub checked: bool,
}

#[derive(Debug, Clone)]
pub enum EnvEvent {
    Detected {
        node: Option<NodeRuntimeInfo>,
        package_manager: Option<PackageManager>,
        available_package_managers: Vec<PackageManager>,
        jiti_path: Option<PathBuf>,
    },
}

impl DomainModel for EnvModel {
    type Event = EnvEvent;

    fn apply(&mut self, event: Self::Event) -> bool {
        match event {
            EnvEvent::Detected {
                node,
                package_manager,
                available_package_managers,
                jiti_path,
            } => {
                self.node = node;
                self.package_manager = package_manager;
                self.available_package_managers = available_package_managers;
                self.jiti_path = jiti_path;
                self.checked = true;
                true
            }
        }
    }
}
