mod entry;

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[allow(dead_code)]
#[cfg(target_os = "android")]
#[no_mangle]
#[tokio::main]
async fn android_main(app: AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    use doufu_core::user_event::UserEvent;
    use doufu_core::winit::event_loop::EventLoop;

    doufu_pal::logger::setup();
    doufu_pal::config::setup().await;

    let event_loop: EventLoop<UserEvent> = winit::event_loop::EventLoopBuilder::with_user_event()
        .with_android_app(app)
        .build()
        .unwrap();
    entry::main_entry(event_loop).await;
}

#[cfg(web)]
#[cfg_attr(web, wasm_bindgen::prelude::wasm_bindgen)]
pub async fn doufu_init(element_id: &str) {
    doufu_pal::logger::setup();
    doufu_pal::config::setup().await;

    let event_loop = doufu_core::surface::create_eventloop();
    entry::main_entry(event_loop, element_id).await;
}
