#[macro_use]
mod macros;
mod internals;
mod utils;

use v8::{
    script_compiler::{self, Source},
    CallbackScope, Context, ContextScope, FixedArray, HandleScope, Isolate, Local, Module,
    OwnedIsolate, Promise, PromiseResolver, ScriptOrModule, ScriptOrigin, String,
};

use crate::v8::{internals::setup, utils::IntoV8};

pub struct V8 {
    isolate: OwnedIsolate,
}

impl V8 {
    pub fn init() -> Self {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();

        let mut isolate = Isolate::new(Default::default());

        isolate.set_host_import_module_dynamically_callback(dynamic_import_callback);

        V8 { isolate }
    }

    pub fn run(&mut self) {
        let scope = &mut HandleScope::new(&mut self.isolate);
        let global_context = Context::new(scope);
        let scope = &mut ContextScope::new(scope, global_context);

        // install internal objects to global
        let global = global_context.global(scope);
        setup(scope, &global);

        let code = String::new(
            scope,
            "import a from 'aaa';console.log('aaa'); 'a' + 'b';a();",
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
        let mut result = module.evaluate(scope).unwrap();
        if result.is_promise() {
            let resolver = PromiseResolver::new(scope).unwrap();
            let promise = resolver.get_promise(scope);
            resolver.resolve(scope, result);
            result = promise.result(scope);
        }
        let result = result.to_string(scope).unwrap();
        println!("result: {}", result.to_rust_string_lossy(scope));
    }
}

fn import_module<'s>(
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

fn module_resolve_callback<'a>(
    context: Local<'a, Context>,
    specifier: Local<'a, String>,
    import_assertions: Local<'a, FixedArray>,
    referrer: Local<'a, Module>,
) -> Option<Local<'a, Module>> {
    let scope = &mut unsafe { CallbackScope::new(context) };
    // let scope = &mut HandleScope::new(scope);
    let (module, _) = import_module(scope, specifier, import_assertions);
    Some(module)
}

extern "C" fn dynamic_import_callback(
    context: Local<Context>,
    _referrer: Local<ScriptOrModule>,
    _specifier: Local<String>,
    import_assertions: Local<FixedArray>,
) -> *mut Promise {
    let scope = &mut unsafe { CallbackScope::new(context) };
    let scope = &mut HandleScope::new(scope);
    let (_, promise) = import_module(scope, _specifier, import_assertions);
    return &*promise as *const _ as *mut _;
}
