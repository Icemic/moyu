//! Project scaffolding service (`init`).

use std::fs;
use std::path::Path;

use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTemplate {
    Basic,
    Advanced,
}

pub fn validate_project_name(name: &str) -> Result<()> {
    let bytes = name.as_bytes();
    let valid = !bytes.is_empty()
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-'));
    if !valid {
        bail!("Invalid project name. Use lowercase letters, numbers, hyphens, and dots.");
    }
    Ok(())
}

pub fn create_project(name: &str, template: ProjectTemplate, target_dir: &Path) -> Result<()> {
    validate_project_name(name)?;
    if target_dir.exists() {
        bail!("Directory \"{}\" already exists.", target_dir.display());
    }

    fs::create_dir_all(target_dir.join("src"))?;
    fs::create_dir_all(target_dir.join("assets"))?;

    let index_json = serde_json::json!({
        "initialSurfaceSize": "1280x720",
        "stageSize": "1920x1080",
        "windowTitle": name,
        "entryFilename": "index.js"
    });
    fs::write(
        target_dir.join("index.json"),
        serde_json::to_string_pretty(&index_json)? + "\n",
    )?;

    let moyu_json = serde_json::json!({
        "engine": {
            "channel": "stable"
        }
    });
    fs::write(
        target_dir.join("moyu.json"),
        serde_json::to_string_pretty(&moyu_json)? + "\n",
    )?;

    let template_label = match template {
        ProjectTemplate::Basic => "basic",
        ProjectTemplate::Advanced => "advanced",
    };

    let package_json = serde_json::json!({
        "name": name,
        "private": true,
        "version": "0.1.0",
        "type": "module",
        "description": format!("Moyu visual novel project ({template_label})"),
        "scripts": {
            "build": "echo \"Configure your bundler build script\""
        }
    });
    fs::write(
        target_dir.join("package.json"),
        serde_json::to_string_pretty(&package_json)? + "\n",
    )?;

    fs::write(
        target_dir.join("src").join("index.ts"),
        "// Entry point for your Moyu visual novel.\nconsole.log('Hello, Moyu!');\n",
    )?;

    Ok(())
}
