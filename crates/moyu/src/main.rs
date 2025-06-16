#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod entry;
mod splash;

#[cfg(native)]
#[tokio::main]
async fn main() {
    moyu_pal::logger::setup();
    moyu_pal::config::setup().await;

    #[cfg(debug_assertions)]
    log::debug!("Environtment: {:#?}", moyu_pal::config::get_engine_config());

    let event_loop = moyu_core::surface::create_eventloop();

    entry::main_entry(event_loop).await;
}

#[cfg(web)]
fn main() {
    // Adds this function only to avoid the warning of "`main` function not found"
}
