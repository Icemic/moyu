use log::{error, info};
use std::cell::RefCell;
use std::fs;
use std::rc::Rc;
use std::{path::PathBuf, process::exit};
use v8::{
    script_compiler::{self, Source},
    FixedArray, Global, HandleScope, Local, Module, Promise, PromiseResolver, ScriptOrigin, String,
};

use super::module_resolve_callback;
use crate::v8::utils::try_find_file;
use crate::v8::{state::State, utils::IntoV8};

pub enum ResolvedModule {
    // path to local disk
    Local,
    // url
    Remote,
    // file not exists or other errors
    None,
}

pub fn import_module<'s>(
    scope: &mut HandleScope<'s>,
    referrer_name: Option<std::string::String>,
    referrer_script_id: Option<i32>,
    specifier: std::string::String,
    import_assertions: Option<Local<'s, FixedArray>>,
) -> (Local<'s, Module>, Local<'s, Promise>) {
    // get referrer name which is required to load local module
    let mut actual_referrer_name = std::string::String::new();
    if let Some(referrer_name) = referrer_name {
        actual_referrer_name.push_str(&referrer_name);
    } else if let Some(referrer_script_id) = referrer_script_id {
        let state = scope.get_slot_mut::<Rc<RefCell<State>>>().unwrap().clone();
        let state = state.borrow_mut();
        if let Some(referrer_name) = state.get_module_referrer_name(referrer_script_id) {
            actual_referrer_name.push_str(&referrer_name)
        }
    }

    // force quit if a module cannot be loaded
    if actual_referrer_name.is_empty() {
        error!(
            "[module] cannot load module '{}', lack of referrer name.",
            specifier
        );
        exit(-1);
    }

    // resolve to absolute referrer path
    let (module_type, module_referrer) = resolve_module(&specifier, &actual_referrer_name);

    let state = scope.get_slot_mut::<Rc<RefCell<State>>>().unwrap().clone();
    let mut state = state.borrow_mut();

    // check cache or load new module
    if let Some(module) = state.get_module(&module_referrer) {
        let module = Local::new(scope, module);

        let resolver = PromiseResolver::new(scope).unwrap();
        let promise = resolver.get_promise(scope);
        // TODO: Is undefined ok?
        let undefined = v8::undefined(scope).into();
        resolver.resolve(scope, undefined);

        info!(
            "[module] module '{}' loaded from '{}' (use cache)",
            specifier, module_referrer
        );

        return (module, promise);
    }

    let code = match module_type {
        ResolvedModule::Local => {
            let code = read_code_local(&module_referrer);
            info!(
                "[module] module '{}' loaded from '{}'",
                specifier, module_referrer
            );
            code
        }
        ResolvedModule::Remote => {
            let code = read_code_remote(&module_referrer);
            info!(
                "[module] module '{}' loaded from '{}'",
                specifier, module_referrer
            );
            code
        }
        ResolvedModule::None => {
            error!(
                "[module] cannot load module '{}', file not exists.",
                specifier
            );
            exit(-1);
        }
    };

    let resource_name = module_referrer.clone().into_v8(scope).into();
    let source_map_url = "".into_v8(scope).into();

    let origin = ScriptOrigin::new(
        scope,
        resource_name,
        0,
        0,
        false,
        0,
        source_map_url,
        false,
        false,
        true,
    );

    // get source instance
    let code = code.into_v8(scope);
    let source = Source::new(code, Some(&origin));

    // compile module
    let module = script_compiler::compile_module(scope, source).unwrap();

    // save to state
    let script_id = module.script_id().unwrap();
    let global_module = Global::new(scope, module);
    state.save_module(&module_referrer, global_module);
    state.save_module_referrer_name(script_id, module_referrer);

    // for `module_resolve_callback` below may cause a recursive calling while state is still borrowed.
    drop(state);

    // instantiate and run module code
    module
        .instantiate_module(scope, module_resolve_callback)
        .unwrap();
    let result = module.evaluate(scope).unwrap();

    // resolve promise of module loading
    let resolver = PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    resolver.resolve(scope, result);

    // TODO: error handling?

    (module, promise)
}

pub fn resolve_module(
    specifier: &str,
    referrer_name: &str,
) -> (ResolvedModule, std::string::String) {
    if specifier.starts_with(".") {
        let path = PathBuf::from(referrer_name).with_file_name("");

        if let Some(filename) =
            try_find_file(&path, specifier, vec!["ts", "tsx", "mjs", "jsx", "js"])
        {
            return (
                ResolvedModule::Local,
                filename.to_str().unwrap().to_string(),
            );
        }

        return (ResolvedModule::None, "".to_string());
    }

    // treat others as remote modules (just like modules in `node_modules` for nodejs)
    let mut path = std::string::String::new();
    path.push_str("https://esm.sh/");
    path.push_str(specifier);

    if path.contains('?') {
        path.push_str("&target=es2020");
    } else {
        path.push_str("?target=es2020");
    }

    return (ResolvedModule::Remote, path);
}

pub fn read_code_local(filename: &std::string::String) -> std::string::String {
    match fs::read_to_string(filename) {
        Ok(data) => data,
        Err(err) => {
            // force quit if a module cannot be loaded
            error!(
                "[module] cannot load module, something went wrong at reading file '{}' ({}).",
                filename,
                err.to_string()
            );
            exit(-1);
        }
    }
}

pub fn read_code_remote(url: &std::string::String) -> std::string::String {
    todo!("pull module from remote");
}
