pub use web_time::*;

/// Sleep for the specified duration.
pub async fn sleep(duration: std::time::Duration) {
    use wasm_bindgen_futures::JsFuture;

    let promise = web_sys::js_sys::Promise::new(&mut |resolve, _reject| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                duration.as_millis() as i32,
            )
            .unwrap();
    });
    JsFuture::from(promise).await.unwrap();
}

/// Wait for the next animation frame.
pub async fn wait_animation_frame() {
    use wasm_bindgen_futures::JsFuture;

    let promise = web_sys::js_sys::Promise::new(&mut |resolve, _reject| {
        web_sys::window()
            .unwrap()
            .request_animation_frame(&resolve)
            .unwrap();
    });
    JsFuture::from(promise).await.unwrap();
}
