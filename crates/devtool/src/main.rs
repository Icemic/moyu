#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod cli;
mod config;
mod gui;
mod model;
mod service;

use clap::Parser;

fn should_use_cli(args: &[std::ffi::OsString]) -> bool {
    if args.len() <= 1 {
        return false;
    }

    let mut has_explicit_gui = false;
    for arg in &args[1..] {
        let s = arg.to_string_lossy();
        if s == "--no-gui" || s == "-h" || s == "--help" || s == "-V" || s == "--version" {
            return true;
        }
        if s == "--gui" {
            has_explicit_gui = true;
            continue;
        }
        if !s.starts_with('-') {
            return true;
        }
    }

    !has_explicit_gui
}

fn main() {
    let raw_args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if !should_use_cli(&raw_args) {
        std::process::exit(gui::run());
    }

    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }

    let parsed = cli::Cli::parse_from(&raw_args);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio runtime");

    let code = rt.block_on(cli::run(parsed));
    std::process::exit(code);
}
