use std::ffi::c_void;

use android_activity::AndroidApp;
use jni::objects::{JObject, JString};
use jni::{JavaVM, JValue, jni_sig, jni_str};

use crate::visible_hand::{InvisibleHand, VisibleHand};

/// Intent extra key that overrides the engine entry file.
///
/// It lets a host application point the engine at an entry that only exists at
/// runtime (for example a local preview server) without repackaging the APK.
pub const ENTRY_EXTRA: &str = "moyu.entry";

static ANDROID_APP: InvisibleHand<AndroidApp> = InvisibleHand::new();

pub fn setup_android(app: &AndroidApp) -> VisibleHand<AndroidApp> {
    ANDROID_APP.set(app.clone()).expect("Failed to set handle.");
    ANDROID_APP.intervent()
}

pub fn get_android_app<'a>() -> &'a AndroidApp {
    ANDROID_APP.get()
}

/// Reads the [`ENTRY_EXTRA`] string extra from the activity's launch intent.
pub fn intent_entry() -> Option<String> {
    intent_string_extra(ENTRY_EXTRA)
}

fn intent_string_extra(key: &str) -> Option<String> {
    let app = get_android_app();
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let activity = app.activity_as_ptr();

    vm.attach_current_thread(|env| read_string_extra(env, activity, key))
        .ok()
        .flatten()
}

fn read_string_extra(
    env: &mut jni::Env<'_>,
    activity: *mut c_void,
    key: &str,
) -> jni::errors::Result<Option<String>> {
    let activity = unsafe { JObject::from_raw(&*env, activity.cast()) };

    let intent = env
        .call_method(
            &activity,
            jni_str!("getIntent"),
            jni_sig!("()Landroid/content/Intent;"),
            &[],
        )?
        .l()?;

    if intent.is_null() {
        return Ok(None);
    }

    let key = env.new_string(key)?;
    let value = env
        .call_method(
            &intent,
            jni_str!("getStringExtra"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
            &[JValue::Object(&key)],
        )?
        .l()?;

    if value.is_null() {
        return Ok(None);
    }

    let value = env.cast_local::<JString>(value)?;

    Ok(Some(value.mutf8_chars(env)?.to_str().into_owned()))
}
