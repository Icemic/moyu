//! Cross-platform detection and launching for code editors, terminals, AI tools, and file manager.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum LauncherKind {
    Editor,
    Terminal,
    Ai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum LaunchStrategy {
    /// `<command> <project_root>` (VS Code, Cursor, Antigravity, etc.)
    OpenPathArg,
    /// Windows Terminal: `wt.exe -d <project_root>`
    #[cfg(windows)]
    WindowsTerminal,
    /// Windows console shell (`pwsh`, `powershell`) in a new console window at `project_root`.
    #[cfg(windows)]
    WindowsConsoleShell,
    /// Git Bash: `git-bash.exe --cd=<project_root>`
    #[cfg(windows)]
    GitBash,
    /// Codex Desktop App via bundled/PATH `codex.exe app <project_root>`.
    #[cfg(windows)]
    CodexApp,
    /// WezTerm: `wezterm start --cwd <project_root>`
    WezTerm,
    /// Alacritty: `alacritty --working-directory <project_root>`
    Alacritty,
    /// macOS `open -a <app_path> <project_root>`
    #[cfg(target_os = "macos")]
    MacOsOpenApp,
    /// Linux terminal with `--working-directory=<dir>` or `--workdir`
    #[cfg(all(unix, not(target_os = "macos")))]
    LinuxTerminalDirFlag(&'static str),
    /// Interactive CLI tool (Codex CLI, Claude Code, Gemini CLI, OpenCode) launched in a terminal.
    TerminalCli,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct DetectedEditor {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: LauncherKind,
    pub command: PathBuf,
    strategy: LaunchStrategy,
}

struct LauncherSpec {
    id: &'static str,
    name: &'static str,
    kind: LauncherKind,
    strategy: LaunchStrategy,
    path_bins: &'static [&'static str],
    #[cfg(windows)]
    win_local_app_data: &'static [&'static str],
    #[cfg(windows)]
    win_program_files: &'static [&'static str],
    /// `(package_prefix, executable_name)` pairs under `%LOCALAPPDATA%\Microsoft\WindowsApps`.
    #[cfg(windows)]
    win_apps_prefixes: &'static [(&'static str, &'static str)],
    #[cfg(target_os = "macos")]
    macos_apps: &'static [&'static str],
}

impl LauncherSpec {
    const fn base(
        id: &'static str,
        name: &'static str,
        kind: LauncherKind,
        strategy: LaunchStrategy,
        path_bins: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            name,
            kind,
            strategy,
            path_bins,
            #[cfg(windows)]
            win_local_app_data: &[],
            #[cfg(windows)]
            win_program_files: &[],
            #[cfg(windows)]
            win_apps_prefixes: &[],
            #[cfg(target_os = "macos")]
            macos_apps: &[],
        }
    }
}

const LAUNCHERS: &[LauncherSpec] = &[
    // --- Editors ---
    LauncherSpec {
        #[cfg(windows)]
        win_local_app_data: &[
            r"Programs\Microsoft VS Code\bin\code.cmd",
            r"Programs\Microsoft VS Code\Code.exe",
        ],
        #[cfg(windows)]
        win_program_files: &[
            r"Microsoft VS Code\bin\code.cmd",
            r"Microsoft VS Code\Code.exe",
        ],
        #[cfg(target_os = "macos")]
        macos_apps: &["/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code"],
        ..LauncherSpec::base(
            "vscode",
            "VS Code",
            LauncherKind::Editor,
            LaunchStrategy::OpenPathArg,
            &["code"],
        )
    },
    LauncherSpec {
        #[cfg(windows)]
        win_local_app_data: &[
            r"Programs\Microsoft VS Code Insiders\bin\code-insiders.cmd",
            r"Programs\Microsoft VS Code Insiders\Code - Insiders.exe",
        ],
        #[cfg(windows)]
        win_program_files: &[
            r"Microsoft VS Code Insiders\bin\code-insiders.cmd",
            r"Microsoft VS Code Insiders\Code - Insiders.exe",
        ],
        #[cfg(target_os = "macos")]
        macos_apps: &[
            "/Applications/Visual Studio Code - Insiders.app/Contents/Resources/app/bin/code",
        ],
        ..LauncherSpec::base(
            "vscode-insiders",
            "VS Code Insiders",
            LauncherKind::Editor,
            LaunchStrategy::OpenPathArg,
            &["code-insiders"],
        )
    },
    LauncherSpec {
        #[cfg(windows)]
        win_local_app_data: &[
            r"Programs\cursor\resources\app\bin\cursor.cmd",
            r"Programs\cursor\Cursor.exe",
        ],
        #[cfg(windows)]
        win_program_files: &[r"cursor\resources\app\bin\cursor.cmd", r"cursor\Cursor.exe"],
        #[cfg(target_os = "macos")]
        macos_apps: &["/Applications/Cursor.app/Contents/Resources/app/bin/cursor"],
        ..LauncherSpec::base(
            "cursor",
            "Cursor",
            LauncherKind::Editor,
            LaunchStrategy::OpenPathArg,
            &["cursor"],
        )
    },
    LauncherSpec {
        #[cfg(windows)]
        win_local_app_data: &[
            r"Programs\Windsurf\bin\windsurf.cmd",
            r"Programs\Windsurf\Windsurf.exe",
        ],
        #[cfg(windows)]
        win_program_files: &[r"Windsurf\bin\windsurf.cmd", r"Windsurf\Windsurf.exe"],
        #[cfg(target_os = "macos")]
        macos_apps: &["/Applications/Windsurf.app/Contents/Resources/app/bin/windsurf"],
        ..LauncherSpec::base(
            "windsurf",
            "Windsurf",
            LauncherKind::Editor,
            LaunchStrategy::OpenPathArg,
            &["windsurf"],
        )
    },
    LauncherSpec {
        #[cfg(windows)]
        win_local_app_data: &[r"Programs\Zed\Zed.exe", r"Zed\Zed.exe"],
        #[cfg(windows)]
        win_program_files: &[r"Zed\Zed.exe"],
        #[cfg(target_os = "macos")]
        macos_apps: &["/Applications/Zed.app/Contents/MacOS/cli"],
        ..LauncherSpec::base(
            "zed",
            "Zed",
            LauncherKind::Editor,
            LaunchStrategy::OpenPathArg,
            &["zed"],
        )
    },
    LauncherSpec {
        #[cfg(windows)]
        win_local_app_data: &[r"Programs\WebStorm\bin\webstorm64.exe"],
        #[cfg(target_os = "macos")]
        macos_apps: &["/Applications/WebStorm.app/Contents/MacOS/webstorm"],
        ..LauncherSpec::base(
            "webstorm",
            "WebStorm",
            LauncherKind::Editor,
            LaunchStrategy::OpenPathArg,
            &["webstorm", "webstorm64"],
        )
    },
    LauncherSpec {
        #[cfg(windows)]
        win_local_app_data: &[r"Programs\Trae\bin\trae.cmd", r"Programs\Trae\Trae.exe"],
        #[cfg(windows)]
        win_program_files: &[r"Trae\bin\trae.cmd", r"Trae\Trae.exe"],
        #[cfg(target_os = "macos")]
        macos_apps: &["/Applications/Trae.app/Contents/Resources/app/bin/trae"],
        ..LauncherSpec::base(
            "trae",
            "Trae",
            LauncherKind::Editor,
            LaunchStrategy::OpenPathArg,
            &["trae"],
        )
    },
    // --- Terminals ---
    #[cfg(windows)]
    LauncherSpec {
        win_local_app_data: &[r"Microsoft\WindowsApps\wt.exe"],
        ..LauncherSpec::base(
            "windows-terminal",
            "Windows Terminal",
            LauncherKind::Terminal,
            LaunchStrategy::WindowsTerminal,
            &["wt"],
        )
    },
    #[cfg(windows)]
    LauncherSpec {
        win_program_files: &[r"PowerShell\7\pwsh.exe"],
        ..LauncherSpec::base(
            "pwsh",
            "PowerShell 7",
            LauncherKind::Terminal,
            LaunchStrategy::WindowsConsoleShell,
            &["pwsh"],
        )
    },
    #[cfg(windows)]
    LauncherSpec::base(
        "powershell",
        "PowerShell",
        LauncherKind::Terminal,
        LaunchStrategy::WindowsConsoleShell,
        &["powershell"],
    ),
    #[cfg(windows)]
    LauncherSpec {
        win_local_app_data: &[r"Programs\Git\git-bash.exe"],
        win_program_files: &[r"Git\git-bash.exe"],
        ..LauncherSpec::base(
            "git-bash",
            "Git Bash",
            LauncherKind::Terminal,
            LaunchStrategy::GitBash,
            &["git-bash"],
        )
    },
    #[cfg(target_os = "macos")]
    LauncherSpec {
        macos_apps: &["/System/Applications/Utilities/Terminal.app"],
        ..LauncherSpec::base(
            "macos-terminal",
            "Terminal",
            LauncherKind::Terminal,
            LaunchStrategy::MacOsOpenApp,
            &[],
        )
    },
    #[cfg(target_os = "macos")]
    LauncherSpec {
        macos_apps: &["/Applications/iTerm.app"],
        ..LauncherSpec::base(
            "iterm2",
            "iTerm2",
            LauncherKind::Terminal,
            LaunchStrategy::MacOsOpenApp,
            &[],
        )
    },
    #[cfg(target_os = "macos")]
    LauncherSpec {
        macos_apps: &["/Applications/Warp.app"],
        ..LauncherSpec::base(
            "warp",
            "Warp",
            LauncherKind::Terminal,
            LaunchStrategy::MacOsOpenApp,
            &[],
        )
    },
    #[cfg(target_os = "macos")]
    LauncherSpec {
        macos_apps: &["/Applications/Ghostty.app"],
        ..LauncherSpec::base(
            "ghostty",
            "Ghostty",
            LauncherKind::Terminal,
            LaunchStrategy::MacOsOpenApp,
            &[],
        )
    },
    #[cfg(all(unix, not(target_os = "macos")))]
    LauncherSpec::base(
        "gnome-terminal",
        "GNOME Terminal",
        LauncherKind::Terminal,
        LaunchStrategy::LinuxTerminalDirFlag("--working-directory"),
        &["gnome-terminal"],
    ),
    #[cfg(all(unix, not(target_os = "macos")))]
    LauncherSpec::base(
        "konsole",
        "Konsole",
        LauncherKind::Terminal,
        LaunchStrategy::LinuxTerminalDirFlag("--workdir"),
        &["konsole"],
    ),
    #[cfg(all(unix, not(target_os = "macos")))]
    LauncherSpec::base(
        "kitty",
        "Kitty",
        LauncherKind::Terminal,
        LaunchStrategy::LinuxTerminalDirFlag("--directory"),
        &["kitty"],
    ),
    LauncherSpec {
        #[cfg(windows)]
        win_program_files: &[r"WezTerm\wezterm.exe"],
        #[cfg(target_os = "macos")]
        macos_apps: &["/Applications/WezTerm.app/Contents/MacOS/wezterm"],
        ..LauncherSpec::base(
            "wezterm",
            "WezTerm",
            LauncherKind::Terminal,
            LaunchStrategy::WezTerm,
            &["wezterm"],
        )
    },
    LauncherSpec {
        #[cfg(windows)]
        win_program_files: &[r"Alacritty\alacritty.exe"],
        #[cfg(target_os = "macos")]
        macos_apps: &["/Applications/Alacritty.app/Contents/MacOS/alacritty"],
        ..LauncherSpec::base(
            "alacritty",
            "Alacritty",
            LauncherKind::Terminal,
            LaunchStrategy::Alacritty,
            &["alacritty"],
        )
    },
    // --- AI Apps & CLIs ---
    LauncherSpec {
        #[cfg(windows)]
        win_local_app_data: &[
            r"Programs\Antigravity\bin\antigravity.cmd",
            r"Programs\Antigravity\Antigravity.exe",
        ],
        #[cfg(windows)]
        win_program_files: &[
            r"Antigravity\bin\antigravity.cmd",
            r"Antigravity\Antigravity.exe",
        ],
        #[cfg(target_os = "macos")]
        macos_apps: &["/Applications/Antigravity.app/Contents/Resources/app/bin/antigravity"],
        ..LauncherSpec::base(
            "antigravity",
            "Antigravity",
            LauncherKind::Ai,
            LaunchStrategy::OpenPathArg,
            &["antigravity"],
        )
    },
    #[cfg(windows)]
    LauncherSpec {
        win_local_app_data: &[r"Programs\OpenAI\Codex\bin\codex.exe"],
        win_apps_prefixes: &[("OpenAI.Codex_", r"app\resources\codex.exe")],
        ..LauncherSpec::base(
            "codex-app",
            "Codex App",
            LauncherKind::Ai,
            LaunchStrategy::CodexApp,
            &[],
        )
    },
    #[cfg(target_os = "macos")]
    LauncherSpec {
        macos_apps: &["/Applications/Codex.app"],
        ..LauncherSpec::base(
            "codex-app",
            "Codex App",
            LauncherKind::Ai,
            LaunchStrategy::MacOsOpenApp,
            &[],
        )
    },
    LauncherSpec::base(
        "codex-cli",
        "Codex CLI",
        LauncherKind::Ai,
        LaunchStrategy::TerminalCli,
        &["codex"],
    ),
    LauncherSpec::base(
        "claude-code",
        "Claude Code",
        LauncherKind::Ai,
        LaunchStrategy::TerminalCli,
        &["claude"],
    ),
    LauncherSpec::base(
        "gemini-cli",
        "Gemini CLI",
        LauncherKind::Ai,
        LaunchStrategy::TerminalCli,
        &["gemini"],
    ),
    LauncherSpec::base(
        "opencode",
        "OpenCode",
        LauncherKind::Ai,
        LaunchStrategy::TerminalCli,
        &["opencode"],
    ),
    LauncherSpec::base(
        "pi",
        "Pi",
        LauncherKind::Ai,
        LaunchStrategy::TerminalCli,
        &["pi"],
    ),
    LauncherSpec::base(
        "deepseek-harness",
        "DeepSeek Harness",
        LauncherKind::Ai,
        LaunchStrategy::TerminalCli,
        &["dsh"],
    ),
];

#[allow(dead_code)]
pub fn detect_editors() -> Vec<DetectedEditor> {
    let mut found = Vec::new();
    for spec in LAUNCHERS {
        if let Some(command) = locate_launcher(spec) {
            found.push(DetectedEditor {
                id: spec.id,
                name: spec.name,
                kind: spec.kind,
                command,
                strategy: spec.strategy,
            });
        }
    }
    found
}

fn locate_launcher(spec: &LauncherSpec) -> Option<PathBuf> {
    for bin in spec.path_bins {
        if let Some(path) = find_in_path(bin) {
            return Some(path);
        }
    }

    #[cfg(windows)]
    {
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            let base = PathBuf::from(local_app);
            for rel in spec.win_local_app_data {
                let candidate = base.join(rel);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
            if !spec.win_apps_prefixes.is_empty() {
                for &(prefix, rel_exe) in spec.win_apps_prefixes {
                    if let Some(pkg_root) = find_windows_msix_package_root(prefix) {
                        let candidate = pkg_root.join(rel_exe);
                        if candidate.is_file() {
                            return Some(candidate);
                        }
                    }
                }
                if let Ok(entries) = std::fs::read_dir(base.join("Microsoft").join("WindowsApps")) {
                    for entry in entries.flatten() {
                        let dir_name = entry.file_name();
                        let dir_str = dir_name.to_string_lossy();
                        for &(prefix, exe) in spec.win_apps_prefixes {
                            if dir_str.starts_with(prefix) {
                                let candidate = entry.path().join(exe);
                                if candidate.is_file() {
                                    return Some(candidate);
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Ok(prog_files) = std::env::var("ProgramFiles") {
            let base = PathBuf::from(prog_files);
            for rel in spec.win_program_files {
                let candidate = base.join(rel);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        for app in spec.macos_apps {
            let candidate = PathBuf::from(app);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(windows)]
fn find_windows_msix_package_root(package_prefix: &str) -> Option<PathBuf> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const PACKAGES_KEY: &str = r"HKCU\Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages";

    let list_out = Command::new("reg")
        .args(["query", PACKAGES_KEY, "/f", package_prefix, "/k"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !list_out.status.success() {
        return None;
    }
    let key_path = String::from_utf8_lossy(&list_out.stdout)
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("HKEY_CURRENT_USER\\"))?
        .to_string();

    let val_out = Command::new("reg")
        .args(["query", &key_path, "/v", "PackageRootFolder"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !val_out.status.success() {
        return None;
    }
    for line in String::from_utf8_lossy(&val_out.stdout).lines() {
        if let Some((_, rest)) = line.split_once("REG_SZ") {
            let folder = rest.trim();
            if !folder.is_empty() {
                return Some(PathBuf::from(folder));
            }
        }
    }
    None
}

fn find_in_path(binary: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        #[cfg(windows)]
        {
            for ext in ["cmd", "exe", "bat", ""] {
                let name = if ext.is_empty() {
                    binary.to_string()
                } else {
                    format!("{binary}.{ext}")
                };
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        #[cfg(not(windows))]
        {
            let candidate = dir.join(binary);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Open `dir` in the platform's native file manager (Explorer / Finder / xdg-open).
#[allow(dead_code)]
pub fn open_in_file_manager(dir: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        Command::new("explorer")
            .arg(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("Failed to open Explorer for {}", dir.display()))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("Failed to open Finder for {}", dir.display()))?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("Failed to open file manager for {}", dir.display()))?;
        Ok(())
    }
}

#[allow(dead_code)]
pub fn open_in_editor(editor: &DetectedEditor, project_root: &Path) -> Result<()> {
    match editor.strategy {
        LaunchStrategy::OpenPathArg => spawn_open_path_arg(editor, project_root),
        #[cfg(windows)]
        LaunchStrategy::WindowsTerminal => {
            Command::new(&editor.command)
                .arg("-d")
                .arg(project_root)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .with_context(|| format!("Failed to launch {}", editor.name))?;
            Ok(())
        }
        #[cfg(windows)]
        LaunchStrategy::WindowsConsoleShell => {
            use std::os::windows::process::CommandExt;
            const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
            Command::new(&editor.command)
                .current_dir(project_root)
                .creation_flags(CREATE_NEW_CONSOLE)
                .spawn()
                .with_context(|| format!("Failed to launch {}", editor.name))?;
            Ok(())
        }
        #[cfg(windows)]
        LaunchStrategy::GitBash => {
            Command::new(&editor.command)
                .arg(format!("--cd={}", project_root.display()))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .with_context(|| format!("Failed to launch {}", editor.name))?;
            Ok(())
        }
        #[cfg(windows)]
        LaunchStrategy::CodexApp => {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            Command::new(&editor.command)
                .arg("app")
                .arg(project_root)
                .creation_flags(CREATE_NO_WINDOW)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .with_context(|| {
                    format!(
                        "Failed to launch {} ({})",
                        editor.name,
                        editor.command.display()
                    )
                })?;
            Ok(())
        }
        LaunchStrategy::WezTerm => {
            Command::new(&editor.command)
                .args(["start", "--cwd"])
                .arg(project_root)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .with_context(|| format!("Failed to launch {}", editor.name))?;
            Ok(())
        }
        LaunchStrategy::Alacritty => {
            Command::new(&editor.command)
                .arg("--working-directory")
                .arg(project_root)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .with_context(|| format!("Failed to launch {}", editor.name))?;
            Ok(())
        }
        #[cfg(target_os = "macos")]
        LaunchStrategy::MacOsOpenApp => {
            Command::new("open")
                .arg("-a")
                .arg(&editor.command)
                .arg(project_root)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .with_context(|| format!("Failed to launch {}", editor.name))?;
            Ok(())
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        LaunchStrategy::LinuxTerminalDirFlag(flag) => {
            Command::new(&editor.command)
                .arg(flag)
                .arg(project_root)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .with_context(|| format!("Failed to launch {}", editor.name))?;
            Ok(())
        }
        LaunchStrategy::TerminalCli => spawn_terminal_cli(editor, project_root),
    }
}

fn spawn_open_path_arg(editor: &DetectedEditor, project_root: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let is_script = editor
            .command
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"));

        let mut cmd = if is_script {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(&editor.command).arg(project_root);
            c.creation_flags(CREATE_NO_WINDOW);
            c
        } else {
            let mut c = Command::new(&editor.command);
            c.arg(project_root);
            c
        };
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| {
                format!(
                    "Failed to launch {} ({})",
                    editor.name,
                    editor.command.display()
                )
            })?;
        Ok(())
    }

    #[cfg(not(windows))]
    {
        Command::new(&editor.command)
            .arg(project_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| {
                format!(
                    "Failed to launch {} ({})",
                    editor.name,
                    editor.command.display()
                )
            })?;
        Ok(())
    }
}

/// Launch an interactive AI CLI (e.g. `codex`, `claude`, `gemini`) inside a new terminal window.
fn spawn_terminal_cli(editor: &DetectedEditor, project_root: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

        if let Some(wt) = find_in_path("wt") {
            Command::new(wt)
                .arg("-d")
                .arg(project_root)
                .args(["cmd", "/K"])
                .arg(&editor.command)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .with_context(|| format!("Failed to launch {} in Windows Terminal", editor.name))?;
            return Ok(());
        }

        Command::new("cmd")
            .arg("/K")
            .arg(&editor.command)
            .current_dir(project_root)
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()
            .with_context(|| format!("Failed to launch {} in console", editor.name))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        let dir_esc = project_root.display().to_string().replace('\'', "'\\''");
        let cmd_esc = editor.command.display().to_string().replace('\'', "'\\''");
        let script = format!(
            "tell application \"Terminal\" to do script \"cd '{dir_esc}' && '{cmd_esc}'\"\ntell application \"Terminal\" to activate"
        );
        Command::new("osascript")
            .args(["-e", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("Failed to launch {} in Terminal.app", editor.name))?;
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(term) = find_in_path("gnome-terminal") {
            Command::new(term)
                .arg("--working-directory")
                .arg(project_root)
                .arg("--")
                .arg(&editor.command)
                .spawn()
                .with_context(|| format!("Failed to launch {} in GNOME Terminal", editor.name))?;
            return Ok(());
        }
        if let Some(term) = find_in_path("konsole") {
            Command::new(term)
                .arg("--workdir")
                .arg(project_root)
                .arg("-e")
                .arg(&editor.command)
                .spawn()
                .with_context(|| format!("Failed to launch {} in Konsole", editor.name))?;
            return Ok(());
        }
        Command::new("x-terminal-emulator")
            .current_dir(project_root)
            .arg("-e")
            .arg(&editor.command)
            .spawn()
            .with_context(|| format!("Failed to launch {} in terminal", editor.name))?;
        Ok(())
    }
}
