use v8::{
    script_compiler::{self, Source},
    FixedArray, HandleScope, Local, Module, Promise, PromiseResolver, ScriptOrigin, String,
};

use super::module_resolve_callback;
use crate::v8::utils::IntoV8;

pub fn import_module<'s>(
    scope: &mut HandleScope<'s>,
    // referrer: Local<'s, ScriptOrModule>,
    specifier: Local<'s, String>,
    import_assertions: Local<'s, FixedArray>,
) -> (Local<'s, Module>, Local<'s, Promise>) {
    let code = String::new(
        scope,
        "export default function a() { console.log('it\\'s a!') }",
    )
    .unwrap();
    println!("javascript code: {}", code.to_rust_string_lossy(scope));

    let resource_name = "main".into_v8(scope).into();
    let resource_map_name = "".into_v8(scope).into();
    let origin = ScriptOrigin::new(
        scope,
        resource_name,
        0,
        0,
        false,
        0,
        resource_map_name,
        false,
        false,
        true,
    );
    let source = Source::new(code, Some(&origin));

    let module = script_compiler::compile_module(scope, source).unwrap();
    module
        .instantiate_module(scope, module_resolve_callback)
        .unwrap();
    let result = module.evaluate(scope).unwrap();

    let resolver = PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    resolver.resolve(scope, result);

    (module, promise)
}
