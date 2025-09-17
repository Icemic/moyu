#[cfg(target_os = "windows")]
use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};

#[cfg(debug_assertions)]
const LOG_FILTER: &str = "info,moyu=debug,moyu_*=debug,console=debug,wgpu=error";
#[cfg(not(debug_assertions))]
const LOG_FILTER: &str = "warn,moyu=info,moyu_*=info,console=info,wgpu=error";

#[cfg(all(native, not(target_os = "android")))]
pub fn setup() {
    // re-attach console for windows release version,
    // then you can receive logs by executing epic from terminal.
    // this is only needed for Windows release version.
    #[cfg(target_os = "windows")]
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS).ok();
    }

    #[cfg(not(target_arch = "wasm32"))]
    log_panics::init();

    let env = env_logger::Env::default().default_filter_or(LOG_FILTER);
    env_logger::init_from_env(env);
}

#[cfg(target_os = "android")]
pub fn setup() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_filter(
                android_logger::FilterBuilder::new()
                    .parse(LOG_FILTER)
                    .build(),
            ),
    );
}

#[cfg(web)]
pub fn setup() {
    use log::Level;
    #[cfg(debug_assertions)]
    console_log::init_with_level(Level::Debug).expect("failed to setup logger.");
    #[cfg(not(debug_assertions))]
    console_log::init_with_level(Level::Info).expect("failed to setup logger.");
}
