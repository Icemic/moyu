#[macro_use]
mod macros;
mod internals;
mod module;
mod utils;

use v8::{
    script_compiler::{self, Source},
    Context, ContextScope, HandleScope, Isolate, OwnedIsolate, PromiseResolver, ScriptOrigin,
    String,
};

use internals::setup;
use module::dynamic_import_callback;
use utils::IntoV8;

use crate::v8::module::module_resolve_callback;

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
