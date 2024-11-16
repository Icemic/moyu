mod entry;

#[cfg(not(feature = "web"))]
#[tokio::main]
async fn main() {
    hai_pal::config::setup().await;
    hai_pal::logger::setup();

    #[cfg(debug_assertions)]
    log::debug!("Environtment: {:#?}", hai_pal::config::get_engine_config());

    let event_loop = hai_core::surface::create_eventloop();

    entry::main_entry(event_loop);
}
