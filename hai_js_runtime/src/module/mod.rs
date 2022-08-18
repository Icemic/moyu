mod callbacks;
mod types;
mod utils;

pub use callbacks::*;
pub use types::*;
pub use utils::*;

use futures::stream::FuturesUnordered;
use futures::task::AtomicWaker;
use futures::{Future, FutureExt};
use hai_module_compiler::transpile;
use log::{debug, info};
use std::collections::HashMap;
use std::env;
use std::pin::Pin;
use v8::{
    script_compiler::{self, Source},
    Global, HandleScope, Local, Module, ModuleRequest, ModuleStatus, Promise, ScriptOrigin, Value,
};

use crate::utils::IntoV8;

pub struct ModuleLoader {
    pub entry_resolved_specifier: Option<std::string::String>,
    pub module_referrer_names: HashMap<i32, std::string::String>,
    pub module_info_map: HashMap<std::string::String, ModuleInfo>,
    pub pending: FuturesUnordered<
        Pin<
            Box<
                dyn Future<
                    Output = (
                        std::string::String,
                        Result<std::string::String, anyhow::Error>,
                    ),
                >,
            >,
        >,
    >,
    // pub pending_modules: Vec<ModulePending>,
    pub waker: AtomicWaker,
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {
            entry_resolved_specifier: None,
            module_referrer_names: Default::default(),
            module_info_map: Default::default(),
            pending: Default::default(),
            waker: Default::default(),
        }
    }

    pub fn get_resolved_specifier_from_script_id(
        &self,
        script_id: i32,
    ) -> Option<std::string::String> {
        if let Some(v) = self.module_referrer_names.get(&script_id) {
            return Some(v.clone());
        }
        None
    }

    pub fn prepare_from_entry(&mut self) {
        let mut entry_dir = env::var("HAI_ENTRY")
            .unwrap_or(env::current_dir().unwrap().to_str().unwrap().to_string());

        info!("[module] entry '{}'", entry_dir);

        // input shall be a referrer name but entry_dir is a directory, so do some hack
        entry_dir.push_str("./index");
        // start from entry file

        let resolved_specifier = self.pending_module_info(entry_dir, "./index".to_string());
        self.entry_resolved_specifier = Some(resolved_specifier);
        // self.enqueue_module_pending(ModulePendingStatus::Created, &resolved_specifier, None);
    }

    pub fn pending_module_info(
        &mut self,
        // referrer name has been resolved for it's referrer's specifier
        referrer: std::string::String,
        specifier: std::string::String,
    ) -> std::string::String {
        // resolve to absolute referrer path
        let (module_type, resolved_specifier) = resolve_module_specifier(&specifier, &referrer);

        let module_info = ModuleInfo {
            specifier,
            module_referrer: referrer,
            resolved_specifier: resolved_specifier.clone(),
            module_type,
            script_id: None,
            module: None,
            result: None,
        };

        let _resolved_specifier = resolved_specifier.clone();
        let module_type = module_info.module_type.clone();

        let load_fn = async move {
            // TODO: async load code, create module, then modify

            let code = match module_type {
                ModuleType::Local(script_type) => {
                    let mut code = read_code_local(&resolved_specifier).await;
                    // transpile only applies for local code,
                    // for remote code shall be pre-transpiled
                    code = transpile(&code, &script_type).unwrap().code;
                    // print only the first 255 characters
                    debug!("code transpiled\n{}", &code[..(255.min(code.len()))]);

                    // info!(
                    //     "[module] module '{}' loaded from '{}'",
                    //     specifier, resolved_specifier
                    // );
                    Ok(code)
                }
                ModuleType::Remote => {
                    let code = read_code_remote(&resolved_specifier).await;
                    // info!(
                    //     "[module] module '{}' loaded from '{}'",
                    //     specifier, resolved_specifier
                    // );
                    Ok(code)
                }
                ModuleType::None => {
                    // error!(
                    //     "[module] cannot load module '{}', file not exists.",
                    //     specifier
                    // );
                    Err(anyhow::format_err!(""))
                }
            };

            (resolved_specifier, code)
        }
        .boxed_local();

        self.module_info_map
            .insert(_resolved_specifier.clone(), module_info);
        self.pending.push(load_fn);

        // activate poll
        self.waker.wake();

        _resolved_specifier
    }

    pub fn compile_module(
        &mut self,
        scope: &mut HandleScope,
        resolved_specifier: &str,
        code: &str,
    ) {
        let module_info = self
            .module_info_map
            .get_mut(resolved_specifier)
            .expect(format!("cannot find module {}", resolved_specifier).as_str());

        let resource_name = resolved_specifier.into_v8(scope).into();
        let source_map_url = "<internal>".into_v8(scope).into();

        // create source origin
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

        self.module_referrer_names
            .insert(script_id, resolved_specifier.to_string());

        let global_module = Global::new(scope, module);

        module_info.script_id = Some(script_id);
        module_info.module = Some(global_module);

        // pend dependencies' imports
        let module_requests = module.get_module_requests();
        for i in 0..module_requests.length() {
            let module_request: Local<ModuleRequest> =
                module_requests.get(scope, i).unwrap().try_into().unwrap();
            let specifier = module_request.get_specifier().to_rust_string_lossy(scope);
            // resolved_specifier there is dependency's refererr name
            self.pending_module_info(resolved_specifier.to_string(), specifier);
        }
    }

    pub fn instantiate_module<'a>(
        scope: &mut HandleScope,
        module: Global<Module>,
        resolved_specifier: &str,
    ) -> bool {
        let module = Local::new(scope, module);

        if module.get_status() != ModuleStatus::Uninstantiated {
            println!("???? {:?} {}", module.get_status(), resolved_specifier);
            return false;
            // unreachable!(
            //     "cannot instantiate a module '{}' which has been instantiated or errored",
            //     resolved_specifier
            // );
        }

        debug!("[module] instantiate module '{}'", resolved_specifier);

        // instantiate and run module code
        module
            .instantiate_module(scope, module_resolve_callback)
            .unwrap();

        true
    }

    pub fn evaluate_module(
        scope: &mut HandleScope,
        module: Global<Module>,
        resolved_specifier: &str,
    ) -> Option<Global<Value>> {
        let module = Local::new(scope, module);

        if module.get_status() == ModuleStatus::Evaluated {
            return None;
        }

        if module.get_status() != ModuleStatus::Instantiated {
            return None;
        }

        debug!("[module] evaluate module '{}'", resolved_specifier);

        if let Some(result) = module.evaluate(scope) {
            debug!(
                "[module] instantiate module '{}' finished",
                resolved_specifier
            );

            let result2: Local<Promise> = result.clone().try_into().unwrap();

            println!("{:?} {:?}", result2, module.get_status());

            // TODO: error handling?

            // result must be a promise by design
            let result = Global::new(scope, result);

            return Some(result);
        }

        unreachable!("cannot evaluate a module which does not exist.");
    }

    pub fn get_module(&self, resolved_specifier: &str) -> Option<Global<Module>> {
        let module_info = self
            .module_info_map
            .get(resolved_specifier)
            .expect(format!("cannot find module {}", resolved_specifier).as_str());

        module_info.module.clone()
    }

    #[allow(dead_code)]
    pub fn get_module_result(&self, resolved_specifier: &str) -> Option<Global<Value>> {
        let module_info = self
            .module_info_map
            .get(resolved_specifier)
            .expect(format!("cannot find module {}", resolved_specifier).as_str());

        module_info.result.clone()
    }
}
