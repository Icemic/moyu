#[cfg(target_os = "windows")]
use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};

pub mod buffer;

#[cfg(debug_assertions)]
const LOG_FILTER: &str = "info,moyu=debug,moyu_*=debug,console=debug,wgpu=error";
#[cfg(not(debug_assertions))]
const LOG_FILTER: &str = "warn,moyu=info,moyu_*=info,console=info,wgpu=error";

/// Forwards each record to the platform backend and, while the debug bridge has
/// enabled it, into the in-memory buffer.
struct Recorder {
    backend: Backend,
}

impl log::Log for Recorder {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.backend.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if !self.backend.enabled(record.metadata()) {
            return;
        }

        self.backend.log(record);
        buffer::record(record);
    }

    fn flush(&self) {
        self.backend.flush();
    }
}

#[cfg(all(native, not(target_os = "android")))]
type Backend = env_logger::Logger;

#[cfg(target_os = "android")]
type Backend = android_logger::AndroidLogger;

/// `console_log` only exposes forwarding functions, so this stand-in uses the same
/// one its own logger implementation calls.
#[cfg(web)]
struct Backend;

#[cfg(web)]
impl log::Log for Backend {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            console_log::log(record);
        }
    }

    fn flush(&self) {}
}

/// The web build of `log` has no `alloc` feature, so the recorder is registered by
/// reference instead of as a boxed logger.
#[cfg(web)]
static RECORDER: Recorder = Recorder { backend: Backend };

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

    let backend =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(LOG_FILTER))
            .build();

    // Same sequence as `env_logger::init_from_env`, with the recorder in between.
    log::set_max_level(backend.filter());
    log::set_boxed_logger(Box::new(Recorder { backend })).expect("failed to setup logger.");
}

#[cfg(target_os = "android")]
pub fn setup() {
    let config = android_logger::Config::default()
        .with_max_level(log::LevelFilter::Debug)
        .with_filter(
            android_logger::FilterBuilder::new()
                .parse(LOG_FILTER)
                .build(),
        );

    log::set_max_level(log::LevelFilter::Debug);
    log::set_boxed_logger(Box::new(Recorder {
        backend: android_logger::AndroidLogger::new(config),
    }))
    .expect("failed to setup logger.");
}

#[cfg(web)]
pub fn setup() {
    use log::Level;

    #[cfg(debug_assertions)]
    let level = Level::Debug;
    #[cfg(not(debug_assertions))]
    let level = Level::Info;

    log::set_max_level(level.to_level_filter());
    log::set_logger(&RECORDER).expect("failed to setup logger.");
}
