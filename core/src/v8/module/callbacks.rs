use v8::{
    CallbackScope, Context, FixedArray, HandleScope, Local, Module, Promise, ScriptOrModule, String,
};

use super::import_module;

pub fn module_resolve_callback<'a>(
    context: Local<'a, Context>,
    specifier: Local<'a, String>,
    import_assertions: Local<'a, FixedArray>,
    referrer: Local<'a, Module>,
) -> Option<Local<'a, Module>> {
    let scope = &mut unsafe { CallbackScope::new(context) };
    let specifier = specifier.to_rust_string_lossy(scope);
    let (module, _) = import_module(
        scope,
        None,
        referrer.script_id(),
        specifier,
        Some(import_assertions),
    );
    Some(module)
}

pub extern "C" fn dynamic_import_callback(
    context: Local<Context>,
    referrer: Local<ScriptOrModule>,
    specifier: Local<String>,
    import_assertions: Local<FixedArray>,
) -> *mut Promise {
    let scope = &mut unsafe { CallbackScope::new(context) };
    let scope = &mut HandleScope::new(scope);

    let referrer_name = referrer.get_resource_name().to_rust_string_lossy(scope);
    let specifier = specifier.to_rust_string_lossy(scope);
    let (_, promise) = import_module(
        scope,
        Some(referrer_name),
        None,
        specifier,
        Some(import_assertions),
    );
    return &*promise as *const _ as *mut _;
}
