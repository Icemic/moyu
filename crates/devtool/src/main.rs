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

fn should_launch_gui(args: &[std::ffi::OsString]) -> bool {
    args.len() <= 1 || (args.len() == 2 && args[1] == "--gui")
}

fn main() {
    let raw_args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if should_launch_gui(&raw_args) {
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
