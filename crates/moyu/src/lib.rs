mod entry;
mod splash;

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[allow(dead_code)]
#[cfg(target_os = "android")]
#[no_mangle]
#[tokio::main]
async fn android_main(app: AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    use moyu_core::user_event::UserEvent;
    use moyu_core::winit::event_loop::EventLoop;

    moyu_pal::logger::setup();
    moyu_pal::config::setup().await;

    let event_loop: EventLoop<UserEvent> = winit::event_loop::EventLoopBuilder::with_user_event()
        .with_android_app(app)
        .build()
        .unwrap();
    entry::main_entry(event_loop).await;
}

#[cfg(web)]
#[cfg_attr(web, wasm_bindgen::prelude::wasm_bindgen)]
pub async fn moyu_init(element_id: &str) {
    moyu_pal::logger::setup();
    moyu_pal::config::setup().await;

    let event_loop = moyu_core::surface::create_eventloop();
    entry::main_entry(event_loop, element_id).await;
}
