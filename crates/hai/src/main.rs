mod entry;

#[cfg(not(feature = "web"))]
#[tokio::main]
async fn main() {
    hai_pal::env::setup();
    hai_pal::logger::setup();

    let event_loop = hai_core::surface::create_eventloop();

    entry::main_entry(event_loop);
}
