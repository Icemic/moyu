#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod entry;
#[cfg(native)]
mod mimalloc;
mod splash;

#[cfg(desktop)]
#[tokio::main]
async fn main() {
    moyu_pal::logger::setup();
    moyu_pal::config::setup().await;

    let steam_plugin = if let Some(steam) = &moyu_pal::config::get_engine_config().steam {
        if steam.restart_through_client {
            match moyu_steam::restart_app_if_necessary(steam.app_id) {
                Ok(true) => return,
                Ok(false) => {}
                Err(()) if steam.required => {
                    moyu_pal::platform::show_fatal_error_and_exit(
                        "Steam API library is required but could not be loaded.",
                    );
                }
                Err(()) => {
                    log::warn!("Steam API library could not be loaded; continuing without restart");
                }
            }
        }

        // We have to init steam plugin here because Valve requires the Steam DLL to be initialized before
        // creating a window or initializing graphics.
        match moyu_steam::SteamPlugin::new(steam.app_id) {
            Ok(plugin) => Some(plugin),
            Err(error) if steam.required => {
                log::error!("Failed to initialize Steam: {error}");
                moyu_pal::platform::show_fatal_error_and_exit(
                    format!("Failed to initialize Steam: {error}").as_str(),
                );
            }
            Err(error) => {
                log::warn!("Failed to initialize Steam: {error}");
                None
            }
        }
    } else {
        None
    };

    #[cfg(debug_assertions)]
    log::debug!("Environtment: {:#?}", moyu_pal::config::get_engine_config());

    let event_loop = moyu_core::surface::create_eventloop();

    entry::main_entry(event_loop, steam_plugin).await;
}

#[cfg(any(android, web))]
fn main() {
    // Adds this function only to avoid the warning of "`main` function not found"
}
