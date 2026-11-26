//! Node.js / package-manager detection, project build, and Zod schema generation.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::config::managed_node_root;
use crate::model::env::{EnvModel, NodeRuntimeInfo, PackageManager};

const SCHEMA_RUNNER_JS: &str = r#"
import { pathToFileURL } from 'node:url';
import { writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const jitiEntry = process.env.MOYU_JITI_ENTRY;
const inputPath = process.env.MOYU_INPUT_PATH;
const outputPath = process.env.MOYU_OUTPUT_PATH;
const exportName = process.env.MOYU_EXPORT_NAME;

const require = createRequire(import.meta.url);
const jitiMod = require(jitiEntry);
const createJiti = jitiMod.createJiti ?? jitiMod.default?.createJiti ?? jitiMod;

const jiti = createJiti(pathToFileURL(inputPath).href, { interopDefault: true });
const mod = await jiti.import(inputPath);
const schema = mod?.[exportName];

if (!schema || typeof schema !== 'object' || typeof schema.toJSONSchema !== 'function') {
  console.error(
    `Export "${exportName}" not found or does not have a toJSONSchema() method.\n` +
      `Make sure the file exports a Zod schema named "${exportName}".`,
  );
  process.exit(1);
}

const jsonSchema = schema.toJSONSchema({ io: 'input' });
writeFileSync(outputPath, JSON.stringify(jsonSchema, null, 2) + '\n');
"#;

#[allow(dead_code)]
pub fn detect_env(project_root: Option<&Path>) -> EnvModel {
    let node = detect_node();
    let package_manager = project_root.map(detect_package_manager);
    let jiti_path = project_root.and_then(find_jiti_entry);
    EnvModel {
        node,
        package_manager,
        jiti_path,
        checked: true,
    }
}

pub fn detect_node() -> Option<NodeRuntimeInfo> {
    if let Some(info) = probe_node_binary(Path::new("node"), false) {
        return Some(info);
    }

    if let Ok(root) = managed_node_root()
        && root.is_dir()
        && let Ok(entries) = fs::read_dir(&root)
    {
        let mut candidates: Vec<PathBuf> = entries
            .flatten()
            .map(|e| {
                let base = e.path();
                if cfg!(windows) {
                    base.join("node.exe")
                } else {
                    base.join("bin").join("node")
                }
            })
            .filter(|p| p.is_file())
            .collect();
        candidates.sort();
        candidates.reverse();
        for exe in candidates {
            if let Some(info) = probe_node_binary(&exe, true) {
                return Some(info);
            }
        }
    }

    None
}

fn probe_node_binary(executable: &Path, is_managed: bool) -> Option<NodeRuntimeInfo> {
    let out = Command::new(executable).arg("-v").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let major: u32 = version
        .strip_prefix('v')
        .unwrap_or(&version)
        .split('.')
        .next()?
        .parse()
        .ok()?;
    Some(NodeRuntimeInfo {
        executable: executable.to_path_buf(),
        version,
        major,
        is_managed,
    })
}

pub fn require_node() -> Result<NodeRuntimeInfo> {
    detect_node().ok_or_else(|| {
        anyhow::anyhow!("Node.js was not found in PATH or ~/.moyu/node/. Please install Node.js.")
    })
}

pub fn detect_package_manager(project_root: &Path) -> PackageManager {
    let pkg_json = project_root.join("package.json");
    if let Ok(text) = fs::read_to_string(&pkg_json)
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(pm) = val.get("packageManager").and_then(|v| v.as_str())
    {
        if pm.starts_with("yarn") {
            return PackageManager::Yarn;
        }
        if pm.starts_with("pnpm") {
            return PackageManager::Pnpm;
        }
        if pm.starts_with("npm") {
            return PackageManager::Npm;
        }
    }

    if project_root.join("yarn.lock").is_file() {
        PackageManager::Yarn
    } else if project_root.join("pnpm-lock.yaml").is_file() {
        PackageManager::Pnpm
    } else {
        PackageManager::Npm
    }
}

/// Resolve `jiti` from `<project_root>/node_modules/jiti` or ancestor workspace `node_modules`.
pub fn find_jiti_entry(project_root: &Path) -> Option<PathBuf> {
    let mut current = Some(project_root);
    while let Some(dir) = current {
        let candidate = dir.join("node_modules").join("jiti");
        if candidate.is_dir() {
            let pkg_path = candidate.join("package.json");
            if pkg_path.is_file() {
                return Some(candidate);
            }
        }
        current = dir.parent();
    }
    None
}

fn is_pm_available(pm: PackageManager) -> bool {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd.exe");
        c.args(["/c", &format!("{}.cmd", pm.binary_name()), "--version"]);
        c
    } else {
        let mut c = Command::new(pm.binary_name());
        c.arg("--version");
        c
    };
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Run the project's `build` script using the detected package manager.
pub async fn run_project_build(project_root: &Path) -> Result<()> {
    let root = project_root.to_path_buf();
    let detected_pm = detect_package_manager(&root);
    let pm = if detected_pm != PackageManager::Npm && !is_pm_available(detected_pm) {
        eprintln!(
            "warn: {} was not found in PATH; falling back to npm.",
            detected_pm.binary_name()
        );
        PackageManager::Npm
    } else {
        detected_pm
    };
    let node_info = require_node()?;

    tokio::task::spawn_blocking(move || -> Result<()> {
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd.exe");
            c.args(["/c", &format!("{}.cmd", pm.binary_name()), "run", "build"]);
            c
        } else {
            let mut c = Command::new(pm.binary_name());
            c.args(["run", "build"]);
            c
        };

        // Prepend managed node directory if Node is not already from PATH.
        if node_info.is_managed
            && let Some(bin_dir) = node_info.executable.parent()
        {
            let current_path = std::env::var_os("PATH").unwrap_or_default();
            let mut paths = vec![bin_dir.to_path_buf()];
            paths.extend(std::env::split_paths(&current_path));
            if let Ok(joined) = std::env::join_paths(paths) {
                cmd.env("PATH", joined);
            }
        }

        let status = cmd
            .current_dir(&root)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| format!("Failed to launch {} run build", pm.binary_name()))?;

        if !status.success() {
            bail!(
                "Project build exited with code {}",
                status.code().unwrap_or(1)
            );
        }
        Ok(())
    })
    .await?
}

/// Generate JSON Schema by loading the project's TypeScript file via `jiti` in `node_modules`.
pub async fn generate_json_schema(
    project_root: &Path,
    input_path: &Path,
    output_path: &Path,
    export_name: &str,
) -> Result<()> {
    if !input_path.is_file() {
        bail!(
            "Schema source file not found: {}\nMake sure the file exists at the expected path.",
            input_path.display()
        );
    }

    let node = require_node()?;
    let jiti_dir = find_jiti_entry(project_root).ok_or_else(|| {
        anyhow::anyhow!(
            "Could not find \"jiti\" in node_modules under {} or its parent directories.\nPlease install project dependencies first (e.g. yarn / npm install).",
            project_root.display()
        )
    })?;

    let input = fs::canonicalize(input_path).unwrap_or_else(|_| input_path.to_path_buf());
    let output = output_path.to_path_buf();
    let export = export_name.to_string();
    let root = project_root.to_path_buf();

    tokio::task::spawn_blocking(move || -> Result<()> {
        let status = Command::new(&node.executable)
            .args(["--input-type=module", "-e", SCHEMA_RUNNER_JS])
            .current_dir(&root)
            .env("MOYU_JITI_ENTRY", &jiti_dir)
            .env("MOYU_INPUT_PATH", &input)
            .env("MOYU_OUTPUT_PATH", &output)
            .env("MOYU_EXPORT_NAME", &export)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("Failed to execute node schema runner")?;

        if !status.success() {
            bail!(
                "Schema generation failed with exit code {}",
                status.code().unwrap_or(1)
            );
        }
        Ok(())
    })
    .await?
}
