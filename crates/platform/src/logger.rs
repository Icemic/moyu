#[cfg(not(feature = "web"))]
pub fn setup() {
    #[cfg(debug_assertions)]
    let env = env_logger::Env::default().default_filter_or("hai=debug");
    #[cfg(not(debug_assertions))]
    let env = env_logger::Env::default().default_filter_or("hai=warn,hai_runtime::console=debug");
    env_logger::init_from_env(env);
}

#[cfg(feature = "web")]
pub fn setup() {
    use log::Level;
    #[cfg(debug_assertions)]
    console_log::init_with_level(Level::Debug).expect("failed to setup logger.");
    #[cfg(not(debug_assertions))]
    console_log::init_with_level(Level::Info).expect("failed to setup logger.");
}
