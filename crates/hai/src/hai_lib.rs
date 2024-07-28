mod entry;

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[allow(dead_code)]
#[cfg(target_os = "android")]
#[no_mangle]
#[tokio::main]
async fn android_main(app: AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    use hai_core::user_event::UserEvent;
    use hai_core::winit::event_loop::EventLoop;

    // only for test
    std::env::set_var("HAI_ENTRY", "http://localhost:8080/demo.js");

    hai_pal::env::setup();
    hai_pal::logger::setup();

    let event_loop: EventLoop<UserEvent> = winit::event_loop::EventLoopBuilder::with_user_event()
        .with_android_app(app)
        .build()
        .unwrap();
    entry::main_entry(event_loop);
}

#[cfg(feature = "web")]
#[cfg_attr(feature = "web", wasm_bindgen::prelude::wasm_bindgen)]
pub fn wasm_start() {
    hai_pal::env::setup();
    hai_pal::logger::setup();

    let event_loop = hai_core::surface::create_eventloop();
    entry::main_entry(event_loop);
}
