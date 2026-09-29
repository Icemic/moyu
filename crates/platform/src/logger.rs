#[cfg(target_os = "windows")]
use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};

#[cfg(debug_assertions)]
const LOG_FILTER: &str = "info,moyu=debug,moyu_*=debug,console=debug,wgpu=error";
#[cfg(not(debug_assertions))]
const LOG_FILTER: &str = "warn,moyu=info,moyu_*=info,console=info,wgpu=error";

/// Passed to [`setup`] to wrap the platform backend before it is installed.
///
/// The engine passes `moyu_debugger::create_logger` here. What the wrapper does with
/// the records is its own business, which keeps this layer free of that knowledge.
pub type LoggerWrapper = fn(Box<dyn log::Log>) -> Box<dyn log::Log>;

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

#[cfg(all(native, not(target_os = "android")))]
pub fn setup(wrap: LoggerWrapper) {
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

    // Same sequence as `env_logger::init_from_env`, with the wrapper in between.
    log::set_max_level(backend.filter());
    log::set_boxed_logger(wrap(Box::new(backend))).expect("failed to setup logger.");
}

#[cfg(target_os = "android")]
pub fn setup(wrap: LoggerWrapper) {
    let config = android_logger::Config::default()
        .with_max_level(log::LevelFilter::Debug)
        .with_filter(
            android_logger::FilterBuilder::new()
                .parse(LOG_FILTER)
                .build(),
        );

    log::set_max_level(log::LevelFilter::Debug);
    log::set_boxed_logger(wrap(Box::new(android_logger::AndroidLogger::new(config))))
        .expect("failed to setup logger.");
}

/// The web build of `log` has no `alloc` feature, so the wrapper is leaked into the
/// `'static` reference `set_logger` asks for instead of being boxed into the logger;
/// it lives as long as the page does.
#[cfg(web)]
pub fn setup(wrap: LoggerWrapper) {
    use log::Level;

    #[cfg(debug_assertions)]
    let level = Level::Debug;
    #[cfg(not(debug_assertions))]
    let level = Level::Info;

    log::set_max_level(level.to_level_filter());
    log::set_logger(Box::leak(wrap(Box::new(Backend)))).expect("failed to setup logger.");
}
