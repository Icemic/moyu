//! Rich interactive CLI UI primitives powered by `cliclack`, `indicatif`, and `console`.
//!
//! Automatically degrades to clean plain-text stderr output in non-TTY environments (CI / pipes).

use std::fmt;
use std::io::{self, IsTerminal};
use std::time::Duration;

use anyhow::{Result, bail};
use console::style;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

use crate::model::tasks::{LogLevel, TaskEvent};
use crate::service::{CancelToken, ServiceReporter, next_task_id};

/// Sentinel error returned when the user cancels an interactive prompt (Esc / Ctrl+C).
#[derive(Debug)]
pub struct UserCancelled;

impl fmt::Display for UserCancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Operation cancelled by user.")
    }
}

impl std::error::Error for UserCancelled {}

pub fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

fn map_prompt_err<T>(res: io::Result<T>) -> Result<T> {
    match res {
        Ok(v) => Ok(v),
        Err(e) if e.kind() == io::ErrorKind::Interrupted => Err(UserCancelled.into()),
        Err(e) => Err(e.into()),
    }
}

pub fn intro(title: &str) {
    if is_interactive() {
        let _ = cliclack::intro(style(title).bold().cyan().to_string());
    }
}

pub fn outro(message: &str) {
    if is_interactive() {
        let _ = cliclack::outro(style(message).green().to_string());
    } else {
        eprintln!("success: {message}");
    }
}

pub fn outro_cancel(message: &str) {
    if is_interactive() {
        let _ = cliclack::outro_cancel(message);
    } else {
        eprintln!("info: {message}");
    }
}

pub fn log_info(msg: &str) {
    if is_interactive() {
        let _ = cliclack::log::info(msg);
    } else {
        eprintln!("info: {msg}");
    }
}

pub fn log_success(msg: &str) {
    if is_interactive() {
        let _ = cliclack::log::success(msg);
    } else {
        eprintln!("success: {msg}");
    }
}

pub fn log_warn(msg: &str) {
    if is_interactive() {
        let _ = cliclack::log::warning(msg);
    } else {
        eprintln!("warn: {msg}");
    }
}

pub fn log_error(msg: &str) {
    if is_interactive() {
        let _ = cliclack::log::error(msg);
    } else {
        eprintln!("error: {msg}");
    }
}

#[derive(Debug, Clone)]
pub struct SelectItem {
    pub label: String,
    pub hint: String,
}

impl SelectItem {
    pub fn new(label: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            hint: hint.into(),
        }
    }
}

pub fn prompt_input<F>(
    label: &str,
    default: Option<&str>,
    placeholder: Option<&str>,
    validate: F,
) -> Result<String>
where
    F: Fn(&String) -> std::result::Result<(), String> + 'static,
{
    if !is_interactive() {
        bail!("Interactive input required for \"{label}\". Pass the value via CLI arguments.");
    }
    let mut prompt = cliclack::input(label).validate(validate);
    if let Some(d) = default {
        prompt = prompt.default_input(d);
    }
    if let Some(p) = placeholder {
        prompt = prompt.placeholder(p);
    }
    let val: String = map_prompt_err(prompt.interact())?;
    Ok(val.trim().to_string())
}

pub fn prompt_select(label: &str, items: &[SelectItem], initial_idx: usize) -> Result<usize> {
    if !is_interactive() {
        bail!("Interactive selection required for \"{label}\". Pass the option via CLI arguments.");
    }
    if items.is_empty() {
        bail!("No options available for \"{label}\".");
    }
    let initial = initial_idx.min(items.len() - 1);
    let mut prompt = cliclack::select(label).initial_value(initial);
    for (i, item) in items.iter().enumerate() {
        prompt = prompt.item(i, &item.label, &item.hint);
    }
    map_prompt_err(prompt.interact())
}

pub fn prompt_multiselect(
    label: &str,
    items: &[SelectItem],
    initial_indices: &[usize],
) -> Result<Vec<usize>> {
    if !is_interactive() {
        bail!("Interactive selection required for \"{label}\". Pass options via CLI arguments.");
    }
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let mut prompt = cliclack::multiselect(label).required(false);
    if !initial_indices.is_empty() {
        prompt = prompt.initial_values(initial_indices.to_vec());
    }
    for (i, item) in items.iter().enumerate() {
        prompt = prompt.item(i, &item.label, &item.hint);
    }
    map_prompt_err(prompt.interact())
}

pub fn prompt_confirm(label: &str, default: bool) -> Result<bool> {
    if !is_interactive() {
        bail!(
            "Confirmation required for \"{label}\". Re-run in an interactive terminal or pass --yes."
        );
    }
    map_prompt_err(cliclack::confirm(label).initial_value(default).interact())
}

pub fn prompt_password(label: &str) -> Result<String> {
    if !is_interactive() {
        bail!("Password input for \"{label}\" requires an interactive terminal.");
    }
    map_prompt_err(cliclack::password(label).mask('•').interact())
}

/// Run an async operation while displaying a live spinner in interactive terminals.
pub async fn with_spinner<T, F>(message: &str, fut: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    if !io::stderr().is_terminal() {
        eprintln!("info: {message}");
        return fut.await;
    }
    let pb = ProgressBar::with_draw_target(None, ProgressDrawTarget::stderr());
    pb.set_style(spinner_style());
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    let res = fut.await;
    pb.finish_and_clear();
    res
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("  {spinner:.cyan} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
}

fn bytes_bar_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "  {spinner:.cyan} [{bar:28.cyan/dim}] {percent:>3}% · {bytes}/{total_bytes} · {bytes_per_sec} · ETA {eta}",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("█▉▊▋▌▍▎▏ ")
}

fn unknown_bytes_style() -> ProgressStyle {
    ProgressStyle::with_template("  {spinner:.cyan} Downloading {bytes} ({bytes_per_sec})")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
}

fn format_styled_log(level: LogLevel, line: &str) -> String {
    match level {
        LogLevel::Info => format!("  {} {line}", style("ℹ").cyan()),
        LogLevel::Success => format!("  {} {line}", style("✓").green()),
        LogLevel::Warn => format!("  {} {line}", style("!").yellow()),
        LogLevel::Error => format!("  {} {line}", style("✗").red()),
    }
}

/// Create a `ServiceReporter` backed by `indicatif` spinners & byte progress bars on TTY stderr,
/// or clean plain-text lines in non-TTY / CI environments.
pub fn create_cli_reporter() -> (ServiceReporter, tokio::task::JoinHandle<()>) {
    let (tx, rx) = async_channel::unbounded();
    let reporter = ServiceReporter::with_channel(next_task_id(), tx, CancelToken::new());
    let tty = io::stderr().is_terminal();

    let printer = tokio::spawn(async move {
        if !tty {
            while let Ok(ev) = rx.recv().await {
                match ev {
                    TaskEvent::Stage { stage, .. } => eprintln!("info: {stage}"),
                    TaskEvent::LogLine { level, line, .. } => {
                        eprintln!("{}: {line}", level.prefix());
                    }
                    TaskEvent::Started { .. }
                    | TaskEvent::Progress { .. }
                    | TaskEvent::Finished { .. } => {}
                }
            }
            return;
        }

        let pb = ProgressBar::with_draw_target(None, ProgressDrawTarget::stderr());
        pb.set_style(spinner_style());
        let mut active = false;
        let mut bar_mode: Option<Option<u64>> = None; // None = spinner, Some(total) = bytes bar

        while let Ok(ev) = rx.recv().await {
            match ev {
                TaskEvent::Stage { stage, .. } => {
                    if bar_mode.is_some() || !active {
                        pb.reset();
                        pb.set_style(spinner_style());
                        pb.enable_steady_tick(Duration::from_millis(80));
                        bar_mode = None;
                        active = true;
                    }
                    pb.set_message(stage);
                }
                TaskEvent::Progress {
                    downloaded_bytes,
                    total_bytes,
                    ..
                } => {
                    let desired = Some(total_bytes.filter(|&t| t > 0));
                    if bar_mode != desired || !active {
                        pb.reset();
                        if let Some(Some(total)) = desired {
                            pb.set_length(total);
                            pb.set_style(bytes_bar_style());
                        } else {
                            pb.set_style(unknown_bytes_style());
                        }
                        pb.enable_steady_tick(Duration::from_millis(80));
                        bar_mode = desired;
                        active = true;
                    }
                    pb.set_position(downloaded_bytes);
                }
                TaskEvent::LogLine { level, line, .. } => {
                    let formatted = format_styled_log(level, &line);
                    if active {
                        pb.println(formatted);
                    } else {
                        eprintln!("{formatted}");
                    }
                }
                TaskEvent::Started { .. } | TaskEvent::Finished { .. } => {}
            }
        }

        if active {
            pb.finish_and_clear();
        }
    });

    (reporter, printer)
}
