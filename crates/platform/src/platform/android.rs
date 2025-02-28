use crate::visible_hand::{InvisibleHand, VisibleHand};
use android_activity::AndroidApp;

static mut ANDROID_APP: InvisibleHand<AndroidApp> = InvisibleHand::new();

pub fn setup_android(app: &AndroidApp) -> VisibleHand<AndroidApp> {
    unsafe {
        ANDROID_APP.set(app.clone()).expect("Failed to set handle.");
        ANDROID_APP.intervent()
    }
}

pub fn get_android_app<'a>() -> &'a AndroidApp {
    unsafe { ANDROID_APP.get() }
}
