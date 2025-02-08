mod entry;

#[cfg(native)]
#[tokio::main]
async fn main() {
    doufu_pal::logger::setup();
    doufu_pal::config::setup().await;

    #[cfg(debug_assertions)]
    log::debug!(
        "Environtment: {:#?}",
        doufu_pal::config::get_engine_config()
    );

    let event_loop = doufu_core::surface::create_eventloop();

    entry::main_entry(event_loop).await;
}

#[cfg(web)]
fn main() {
    // Adds this function only to avoid the warning of "`main` function not found"
}
