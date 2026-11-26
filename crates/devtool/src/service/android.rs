//! Android project synchronization, icon generation, and Gradle packaging.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::service::{copy_dir_all, http_get_bytes, sha256_hex};

pub const ANDROID_PLATFORM: &str = "android-aarch64";
pub const ANDROID_FORMATS: &[&str] = &[
    "debug-apk",
    "release-apk",
    "release-aab",
    "android-project",
];

const ANDROID_TEMPLATE_VERSION: &str = "0.0.1";
const ANDROID_TEMPLATE_URL: &str =
    "https://github.com/DeepSpaceMill/moyu-android/archive/refs/tags/v0.0.1.zip";
const ANDROID_TEMPLATE_SHA256: &str =
    "ffdac6565403eb99d6ffed3462075ddde1ec43179f44dd0433974cece8f4104c";

#[derive(Debug, Clone)]
pub struct AndroidConfig {
    pub application_id: String,
    pub app_name: String,
    pub version_code: u32,
    pub version_name: String,
    pub orientation: String,
    pub icon_source: String,
    pub icon_bg_color: String,
    pub foreground_padding: f64,
    pub keystore_path: String,
    pub key_alias: String,
}

impl Default for AndroidConfig {
    fn default() -> Self {
        Self {
            application_id: "com.example.game".to_string(),
            app_name: "My Visual Novel".to_string(),
            version_code: 1,
            version_name: "1.0.0".to_string(),
            orientation: "landscape".to_string(),
            icon_source: String::new(),
            icon_bg_color: "#ffffff".to_string(),
            foreground_padding: 0.18,
            keystore_path: String::new(),
            key_alias: String::new(),
        }
    }
}

pub struct PackAndroidOptions<'a> {
    pub project_root: &'a Path,
    pub engine_dir: &'a Path,
    pub runtime_package_dir: &'a Path,
    pub output_dir: &'a Path,
    pub format: &'a str,
    pub video_decoder_library: Option<&'a Path>,
    pub signing_password: Option<&'a str>,
}

struct AndroidSigning {
    keystore_path: PathBuf,
    key_alias: String,
    password: String,
}

pub fn is_android_format(value: Option<&str>) -> bool {
    value.is_some_and(|v| ANDROID_FORMATS.contains(&v))
}

pub async fn pack_android(options: PackAndroidOptions<'_>) -> Result<()> {
    if !is_android_format(Some(options.format)) {
        bail!(
            "Invalid --android-format. Supported values: {}",
            ANDROID_FORMATS.join(", ")
        );
    }

    let config = load_android_config(options.project_root)?;
    validate_android_config(&config)?;

    let template_dir = ensure_android_template(options.project_root).await?;
    let workdir = ensure_android_workdir(options.project_root, &template_dir)?;

    eprintln!("info: Syncing Android project...");
    sync_android_project(
        options.project_root,
        options.engine_dir,
        options.runtime_package_dir,
        &template_dir,
        &workdir,
        &config,
        options.video_decoder_library,
    )?;

    if options.format == "android-project" {
        let output_path = options.output_dir.join("android-project");
        eprintln!(
            "info: Exporting Android Studio project: {}",
            output_path.display()
        );
        copy_android_project(&workdir, &output_path)?;
        return Ok(());
    }

    let signing = if options.format == "debug-apk" {
        None
    } else {
        Some(resolve_android_signing(
            options.project_root,
            &config,
            options.signing_password,
        )?)
    };

    let artifact = run_gradle_build(&workdir, options.format, signing.as_ref()).await?;
    let ext = if options.format == "release-aab" {
        "aab"
    } else {
        "apk"
    };
    let output_path = options
        .output_dir
        .join(format!("game-{}.{ext}", options.format));
    fs::create_dir_all(options.output_dir)?;
    fs::copy(&artifact, &output_path)?;
    eprintln!("success: Android artifact created: {}", output_path.display());
    Ok(())
}

fn load_android_config(project_root: &Path) -> Result<AndroidConfig> {
    let mut cfg = AndroidConfig::default();
    let config_path = project_root.join("moyu.json");
    if !config_path.is_file() {
        return Ok(cfg);
    }
    let text = fs::read_to_string(&config_path)?;
    let raw: serde_json::Value =
        serde_json::from_str(&text).context("Failed to parse moyu.json.")?;

    let Some(android) = raw.get("android").and_then(|v| v.as_object()) else {
        return Ok(cfg);
    };
    if let Some(v) = android.get("applicationId").and_then(|v| v.as_str()) {
        cfg.application_id = v.to_string();
    }
    if let Some(v) = android.get("appName").and_then(|v| v.as_str()) {
        cfg.app_name = v.to_string();
    }
    if let Some(v) = android.get("versionCode") {
        let Some(code) = v.as_u64() else {
            bail!("Android version code must be a positive integer.");
        };
        if code == 0 || code > i32::MAX as u64 {
            bail!("Android version code must be a positive 32-bit integer.");
        }
        cfg.version_code = code as u32;
    }
    if let Some(v) = android.get("versionName").and_then(|v| v.as_str()) {
        cfg.version_name = v.to_string();
    }
    if let Some(v) = android.get("orientation").and_then(|v| v.as_str()) {
        cfg.orientation = v.to_string();
    }
    if let Some(icon) = android.get("icon").and_then(|v| v.as_object()) {
        if let Some(v) = icon.get("source").and_then(|v| v.as_str()) {
            cfg.icon_source = v.to_string();
        }
        if let Some(bg) = icon.get("background").and_then(|v| v.as_object())
            && let Some(v) = bg.get("color").and_then(|v| v.as_str())
        {
            cfg.icon_bg_color = v.to_string();
        }
        if let Some(v) = icon.get("foregroundPadding").and_then(|v| v.as_f64()) {
            cfg.foreground_padding = v;
        }
    }
    if let Some(signing) = android.get("signing").and_then(|v| v.as_object()) {
        if let Some(v) = signing.get("keystorePath").and_then(|v| v.as_str()) {
            cfg.keystore_path = v.to_string();
        }
        if let Some(v) = signing.get("keyAlias").and_then(|v| v.as_str()) {
            cfg.key_alias = v.to_string();
        }
    }

    Ok(cfg)
}

fn validate_android_config(config: &AndroidConfig) -> Result<()> {
    let valid_app_id = {
        let parts: Vec<&str> = config.application_id.split('.').collect();
        parts.len() >= 2
            && parts.iter().all(|p| {
                let mut chars = p.chars();
                chars
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
                    && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
            })
            && !config.application_id.starts_with("ink.momoyu")
    };
    if !valid_app_id {
        bail!("Invalid Android application ID.");
    }
    if config.app_name.trim().is_empty() {
        bail!("Android app name is required.");
    }
    if config.version_code == 0 {
        bail!("Android version code must be a positive integer.");
    }
    if config.version_name.trim().is_empty()
        || config.version_name.contains('\r')
        || config.version_name.contains('\n')
    {
        bail!("Invalid Android version name.");
    }
    if !matches!(
        config.orientation.as_str(),
        "landscape" | "portrait" | "sensor"
    ) {
        bail!("Android orientation must be landscape, portrait, or sensor.");
    }
    let color = config.icon_bg_color.trim();
    let valid_hex = color.len() == 7
        && color.starts_with('#')
        && color[1..].bytes().all(|b| b.is_ascii_hexdigit());
    if !valid_hex {
        bail!("Android icon background color must use #RRGGBB format.");
    }
    if !config.icon_source.trim().is_empty() && Path::new(&config.icon_source).is_absolute() {
        bail!("Android icon source must be relative to the project directory.");
    }
    if !config.keystore_path.trim().is_empty() && Path::new(&config.keystore_path).is_absolute() {
        bail!("Android keystore path must be relative to the project directory.");
    }
    Ok(())
}

async fn ensure_android_template(project_root: &Path) -> Result<PathBuf> {
    let cache_dir = project_root
        .join(".moyu")
        .join("android-template")
        .join(ANDROID_TEMPLATE_VERSION);
    if cache_dir.exists() {
        return Ok(cache_dir);
    }

    eprintln!("info: Downloading Android template {ANDROID_TEMPLATE_VERSION}...");
    let archive_bytes = http_get_bytes(ANDROID_TEMPLATE_URL).await?;
    let actual_sha = sha256_hex(&archive_bytes);
    if actual_sha != ANDROID_TEMPLATE_SHA256 {
        bail!(
            "Android template SHA-256 mismatch.\nExpected: {ANDROID_TEMPLATE_SHA256}\nActual:   {actual_sha}"
        );
    }

    let tmp_dir = project_root
        .join(".moyu")
        .join("android-template")
        .join(format!(".tmp-{ANDROID_TEMPLATE_VERSION}"));
    let cache_clone = cache_dir.clone();

    tokio::task::spawn_blocking(move || -> Result<()> {
        if tmp_dir.exists() {
            fs::remove_dir_all(&tmp_dir)?;
        }
        fs::create_dir_all(&tmp_dir)?;

        let res = (|| -> Result<()> {
            extract_zip_safe(&archive_bytes, &tmp_dir)?;
            let inner = tmp_dir.join(format!("moyu-android-{ANDROID_TEMPLATE_VERSION}"));
            copy_dir_all(&inner, &cache_clone)?;
            Ok(())
        })();

        let _ = fs::remove_dir_all(&tmp_dir);
        res
    })
    .await??;

    eprintln!("success: Android template cached.");
    Ok(cache_dir)
}

fn extract_zip_safe(archive_bytes: &[u8], dest_dir: &Path) -> Result<()> {
    let cursor = std::io::Cursor::new(archive_bytes);
    let mut zip = zip::ZipArchive::new(cursor)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let entry_name = entry.name()?.to_string();
        let raw_name = entry_name.replace('\\', "/");
        let mut clean = PathBuf::new();
        for comp in Path::new(&raw_name).components() {
            match comp {
                Component::Normal(p) => clean.push(p),
                Component::CurDir => {}
                _ => bail!("Unsafe Android template archive entry: {entry_name}"),
            }
        }
        let output_path = dest_dir.join(&clean);
        if entry.is_dir() {
            fs::create_dir_all(&output_path)?;
        } else {
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = File::create(&output_path)?;
            std::io::copy(&mut entry, &mut out)?;
        }
    }
    Ok(())
}

fn ensure_android_workdir(project_root: &Path, template_dir: &Path) -> Result<PathBuf> {
    let workdir = project_root
        .join(".moyu")
        .join("android-build")
        .join(ANDROID_PLATFORM);
    if !workdir.exists() {
        eprintln!("info: Initializing Android work directory...");
        copy_dir_all(template_dir, &workdir)?;
    }
    Ok(workdir)
}

fn sync_android_project(
    project_root: &Path,
    engine_dir: &Path,
    runtime_package_dir: &Path,
    template_dir: &Path,
    workdir: &Path,
    config: &AndroidConfig,
    video_decoder_library: Option<&Path>,
) -> Result<()> {
    let main_dir = workdir.join("app").join("src").join("main");
    let assets_dir = main_dir.join("assets");
    let res_dir = main_dir.join("res");

    if assets_dir.exists() {
        fs::remove_dir_all(&assets_dir)?;
    }
    copy_dir_all(runtime_package_dir, &assets_dir)?;

    let engine_library = engine_dir.join("libmoyu.so");
    if !engine_library.is_file() {
        bail!(
            "Android engine library not found: {}",
            engine_library.display()
        );
    }
    let jni_libs_dir = main_dir.join("jniLibs").join("arm64-v8a");
    fs::create_dir_all(&jni_libs_dir)?;
    fs::copy(&engine_library, jni_libs_dir.join("libmoyu.so"))?;

    let video_dst = jni_libs_dir.join("libmoyu_video.so");
    if video_dst.exists() {
        let _ = fs::remove_file(&video_dst);
    }
    if let Some(lib) = video_decoder_library {
        if !lib.is_file() {
            bail!("Android video decoder library not found: {}", lib.display());
        }
        fs::copy(lib, video_dst)?;
    }

    if res_dir.exists() {
        fs::remove_dir_all(&res_dir)?;
    }
    copy_dir_all(
        &template_dir.join("app").join("src").join("main").join("res"),
        &res_dir,
    )?;

    sync_launcher_icons(project_root, &res_dir, config)?;
    write_android_properties(workdir, config)?;
    write_app_name(&res_dir, &config.app_name)?;
    Ok(())
}

fn sync_launcher_icons(
    project_root: &Path,
    res_dir: &Path,
    config: &AndroidConfig,
) -> Result<()> {
    let source = config.icon_source.trim();
    if source.is_empty() {
        return Ok(());
    }
    let source_path = project_root.join(source);
    if !source_path.is_file() {
        bail!("Android icon source not found: {}", source_path.display());
    }
    if !source.to_ascii_lowercase().ends_with(".png") {
        bail!("Android icon source must be a PNG file.");
    }

    let anydpi_dir = res_dir.join("mipmap-anydpi-v26");
    let drawable_dir = res_dir.join("drawable");
    let drawable_v24_dir = res_dir.join("drawable-v24");
    let drawable_nodpi_dir = res_dir.join("drawable-nodpi");
    for d in [
        &anydpi_dir,
        &drawable_dir,
        &drawable_v24_dir,
        &drawable_nodpi_dir,
    ] {
        fs::create_dir_all(d)?;
    }

    let adaptive_icon = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
        <adaptive-icon xmlns:android=\"http://schemas.android.com/apk/res/android\">\n\
            <background android:drawable=\"@drawable/ic_launcher_background\" />\n\
            <foreground android:drawable=\"@drawable/ic_launcher_foreground\" />\n\
        </adaptive-icon>\n";
    fs::write(anydpi_dir.join("ic_launcher.xml"), adaptive_icon)?;
    fs::write(anydpi_dir.join("ic_launcher_round.xml"), adaptive_icon)?;
    fs::write(
        drawable_dir.join("ic_launcher_background.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
            <shape xmlns:android=\"http://schemas.android.com/apk/res/android\" android:shape=\"rectangle\">\n\
                <solid android:color=\"{}\" />\n\
            </shape>\n",
            config.icon_bg_color.trim()
        ),
    )?;

    let padding = config.foreground_padding.clamp(0.0, 0.5) * 108.0;
    fs::write(
        drawable_v24_dir.join("ic_launcher_foreground.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
            <inset xmlns:android=\"http://schemas.android.com/apk/res/android\"\n\
                android:drawable=\"@drawable/ic_launcher_foreground_base\"\n\
                android:insetLeft=\"{padding:.2}dp\"\n\
                android:insetTop=\"{padding:.2}dp\"\n\
                android:insetRight=\"{padding:.2}dp\"\n\
                android:insetBottom=\"{padding:.2}dp\" />\n"
        ),
    )?;
    fs::copy(
        &source_path,
        drawable_nodpi_dir.join("ic_launcher_foreground_base.png"),
    )?;

    for density in ["mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi"] {
        let dir = res_dir.join(format!("mipmap-{density}"));
        fs::create_dir_all(&dir)?;
        let _ = fs::remove_file(dir.join("ic_launcher.webp"));
        let _ = fs::remove_file(dir.join("ic_launcher_round.webp"));
        fs::copy(&source_path, dir.join("ic_launcher.png"))?;
        fs::copy(&source_path, dir.join("ic_launcher_round.png"))?;
    }

    Ok(())
}

fn write_android_properties(workdir: &Path, config: &AndroidConfig) -> Result<()> {
    let orientation = match config.orientation.as_str() {
        "portrait" => "sensorPortrait",
        "sensor" => "fullSensor",
        _ => "sensorLandscape",
    };
    let version_name = config.version_name.replace('\\', "\\\\");
    fs::write(
        workdir.join("moyu.properties"),
        format!(
            "MOYU_APPLICATION_ID={}\nMOYU_VERSION_CODE={}\nMOYU_VERSION_NAME={version_name}\nMOYU_ORIENTATION={orientation}\n",
            config.application_id, config.version_code
        ),
    )?;
    Ok(())
}

fn write_app_name(res_dir: &Path, app_name: &str) -> Result<()> {
    let escaped = app_name
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    let values_dir = res_dir.join("values");
    fs::create_dir_all(&values_dir)?;
    fs::write(
        values_dir.join("strings.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n    <string name=\"app_name\">{escaped}</string>\n</resources>\n"
        ),
    )?;
    Ok(())
}

fn resolve_android_signing(
    project_root: &Path,
    config: &AndroidConfig,
    signing_password: Option<&str>,
) -> Result<AndroidSigning> {
    if config.keystore_path.trim().is_empty() {
        bail!("Android keystore path is required for release builds.");
    }
    if config.key_alias.trim().is_empty() {
        bail!("Android signing key alias is required for release builds.");
    }
    let keystore_path = project_root.join(&config.keystore_path);
    if !keystore_path.is_file() {
        bail!("Android keystore not found: {}", keystore_path.display());
    }
    let password = signing_password
        .filter(|p| !p.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Android signing password is required for release builds."))?;
    Ok(AndroidSigning {
        keystore_path,
        key_alias: config.key_alias.trim().to_string(),
        password: password.to_string(),
    })
}

async fn run_gradle_build(
    workdir: &Path,
    format: &str,
    signing: Option<&AndroidSigning>,
) -> Result<PathBuf> {
    let task = match format {
        "debug-apk" => ":app:assembleDebug",
        "release-apk" => ":app:assembleRelease",
        "release-aab" => ":app:bundleRelease",
        _ => bail!("Unsupported Gradle format: {format}"),
    };

    let gradlew = workdir.join(if cfg!(windows) {
        "gradlew.bat"
    } else {
        "gradlew"
    });
    if !gradlew.is_file() {
        bail!("Gradle wrapper not found: {}", gradlew.display());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&gradlew)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&gradlew, perms)?;
    }

    let workdir_owned = workdir.to_path_buf();
    let gradlew_str = gradlew.to_string_lossy().to_string();
    let task_owned = task.to_string();
    let signing_env = signing.map(|s| {
        (
            s.keystore_path.to_string_lossy().to_string(),
            s.password.clone(),
            s.key_alias.clone(),
        )
    });

    eprintln!("info: Running Gradle {task}...");
    tokio::task::spawn_blocking(move || -> Result<()> {
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd.exe");
            c.args(["/c", &gradlew_str, &task_owned]);
            c
        } else {
            let mut c = Command::new(&gradlew_str);
            c.arg(&task_owned);
            c
        };

        cmd.current_dir(&workdir_owned)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        if let Some((ks, pw, alias)) = signing_env {
            cmd.env("MOYU_KEYSTORE_PATH", ks)
                .env("MOYU_STORE_PASSWORD", &pw)
                .env("MOYU_KEY_ALIAS", alias)
                .env("MOYU_KEY_PASSWORD", &pw);
        }

        let status = cmd.status().context("Failed to execute Gradle wrapper")?;
        if !status.success() {
            bail!("Gradle build exited with code {}", status.code().unwrap_or(1));
        }
        Ok(())
    })
    .await??;

    let artifact = match format {
        "debug-apk" => workdir.join("app/build/outputs/apk/debug/app-debug.apk"),
        "release-apk" => workdir.join("app/build/outputs/apk/release/app-release.apk"),
        "release-aab" => workdir.join("app/build/outputs/bundle/release/app-release.aab"),
        _ => unreachable!(),
    };
    if !artifact.is_file() {
        bail!(
            "Gradle reported success but artifact was not found: {}",
            artifact.display()
        );
    }
    Ok(artifact)
}

fn copy_android_project(workdir: &Path, output_dir: &Path) -> Result<()> {
    let abs_workdir = fs::canonicalize(workdir).unwrap_or_else(|_| workdir.to_path_buf());
    let abs_output = fs::canonicalize(output_dir).unwrap_or_else(|_| output_dir.to_path_buf());
    if abs_output.starts_with(&abs_workdir) || abs_workdir.starts_with(&abs_output) {
        bail!(
            "Unsafe Android project output path: {}",
            output_dir.display()
        );
    }
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)?;
    }

    let excluded: BTreeSet<&str> = [
        ".git",
        ".gradle",
        ".idea",
        "build",
        "app/build",
        "local.properties",
    ]
    .into_iter()
    .collect();

    for entry in walkdir::WalkDir::new(workdir) {
        let entry = entry?;
        let rel = entry
            .path()
            .strip_prefix(workdir)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        if rel.is_empty() {
            continue;
        }
        if excluded
            .iter()
            .any(|ex| rel == *ex || rel.starts_with(&format!("{ex}/")))
        {
            continue;
        }
        let target = output_dir.join(entry.path().strip_prefix(workdir)?);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
