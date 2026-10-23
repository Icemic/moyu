//! The runtime with no window.
//!
//! This entry assembles the same engine a windowed run does — core, debug bridge,
//! graphics, built-in nodes, plugins and the project script — but drives the frame loop
//! itself instead of leaving it to a platform event loop. The window system reports no
//! surface, so frames stay in a texture the engine owns and every inspection path
//! (screenshots, the debug bridge) works as usual.
//!
//! See `rfcs/2026-10-22-headless-runtime.md`.

mod window_system;

use std::sync::Arc;
use std::time::{Duration, Instant};

use moyu_core::core::{Core, WindowSystemEvent, set_core};
use moyu_pal::platform;

pub use window_system::HeadlessWindowSystem;

/// Interval between two frames. A windowed run follows the monitor; with no monitor to
/// follow, the loop keeps the rate the engine is designed around.
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Run the project without a window until it asks to quit.
///
/// Called from `main` before the windowed entry, so configuration failures take the
/// dialog-free path set up here rather than blocking on a modal nobody can dismiss.
pub async fn run() {
    // Must come first: nothing below can show a dialog to a session with no window.
    platform::disable_error_dialogs();
    platform::set_headless(true);

    let _async_runtime_handle = platform::setup();
    moyu_core::setup();

    moyu_pal::config::setup().await;

    let surface_size = moyu_pal::config::get_engine_config()
        .initial_surface_size
        .as_tuple();

    let window_system = Arc::new(HeadlessWindowSystem::new(surface_size));
    let core = Arc::new(Core::new(window_system.clone()));
    let _core_handle = set_core(core.clone());

    // The debug bridge answers requests about the core, so it starts once the core exists.
    moyu_debugger::start();

    // Everything that awaited is done before the script VM exists: `QuickVM` records its
    // thread when it is created, and the loop below never yields again.
    moyu_app::prepare(&core).await;

    core.sync_surface_size_with_window(false);

    let _vm_handle = match moyu_app::load_user_script(&core, || {
        log::info!("User script loaded.");
    }) {
        Ok(handle) => handle,
        Err(err) => {
            log::error!("{}", err);
            platform::show_fatal_error_and_exit(err.to_string().as_str());
        }
    };

    moyu_app::announce_ready(core.clone(), false);

    // `Frame` wakes the engine's update thread, which renders on its own thread; the
    // loop does not render inline.
    while !window_system.quit_requested() {
        let started = Instant::now();

        core.handle_window_system_event(WindowSystemEvent::Idle);
        core.handle_window_system_event(WindowSystemEvent::Frame);

        std::thread::sleep(FRAME_INTERVAL.saturating_sub(started.elapsed()));
    }
}
