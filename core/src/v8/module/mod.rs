mod callbacks;
mod types;
mod utils;

pub use callbacks::*;
use hai_module_compiler::transpile;
use log::{debug, error, info};
use std::env;
use std::{collections::HashMap, process::exit};
pub use types::*;
pub use utils::*;
use v8::{
    script_compiler::{self, Source},
    Global, HandleScope, Local, Module, PromiseResolver, ScriptOrigin,
};
use v8::{ModuleStatus, Promise, Value};

use self::utils::resolve_module;
use crate::v8::utils::IntoV8;

pub struct ModuleLoader {
    pub module_referrer_names: HashMap<i32, std::string::String>,
    // pub module_map: HashMap<std::string::String, Global<Module>>,
    // pub module_map_promise: HashMap<std::string::String, Global<Promise>>,
    // pub pending_modules: VecDeque<ModuleInfo>,
    pub module_info_map: HashMap<std::string::String, ModuleInfo>,
    pub pending_modules: Vec<ModulePending>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {
            module_referrer_names: Default::default(),
            // module_map: Default::default(),
            // module_map_promise: Default::default(),
            pending_modules: Default::default(),
            module_info_map: Default::default(),
        }
    }

    pub fn setup_entry_module(&mut self) {
        let mut entry_dir = env::var("HAI_ENTRY")
            .unwrap_or(env::current_dir().unwrap().to_str().unwrap().to_string());

        info!("[module] entry '{}'", entry_dir);

        // input shall be a referrer name but entry_dir is a directory, so do some hack
        entry_dir.push_str("./index");
        // start from entry file

        let resolved_specifier = self.create_module_info(entry_dir, "./index".to_string());
        self.enqueue_module_pending(ModulePendingStatus::Created, &resolved_specifier, None);
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

    pub fn create_module_info(
        &mut self,
        // referrer name has been resolved for it's referrer's specifier
        referrer: std::string::String,
        specifier: std::string::String,
    ) -> std::string::String {
        // resolve to absolute referrer path
        let (module_type, resolved_specifier) = resolve_module(&specifier, &referrer);

        // check cache or create new one
        if let Some(_) = self.module_info_map.get(&resolved_specifier) {
            // no-op
        } else {
            let module_info = ModuleInfo {
                specifier,
                module_referrer: referrer,
                resolved_specifier: resolved_specifier.clone(),
                module_type,
                script_id: None,
                module: None,
                result: None,
            };

            self.module_info_map
                .insert(resolved_specifier.clone(), module_info);
        }

        resolved_specifier
    }

    pub fn enqueue_module_pending(
        &mut self,
        status: ModulePendingStatus,
        resolved_specifier: &str,
        promise_resolver: Option<Global<PromiseResolver>>,
    ) {
        let module_pending = ModulePending {
            resolved_specifier: resolved_specifier.to_string(),
            promise_resolver,
            status,
        };

        self.pending_modules.push(module_pending);
    }

    pub fn resolve_module(&mut self, scope: &mut HandleScope, resolved_specifier: &str) -> bool {
        let module_info = self
            .module_info_map
            .get_mut(resolved_specifier)
            .expect(format!("cannot find module {}", resolved_specifier).as_str());

        if let Some(_) = &module_info.module {
            // unreachable!("this module '{}' has been resolved", resolved_specifier);
            return false;
        }

        let resolved_specifier = &module_info.resolved_specifier;
        let module_type = &module_info.module_type;
        let specifier = &module_info.specifier;

        let mut code = match module_type {
            ModuleType::Local(..) => {
                let code = read_code_local(&resolved_specifier);
                info!(
                    "[module] module '{}' loaded from '{}'",
                    specifier, resolved_specifier
                );
                code
            }
            ModuleType::Remote => {
                let code = read_code_remote(&resolved_specifier);
                info!(
                    "[module] module '{}' loaded from '{}'",
                    specifier, resolved_specifier
                );
                code
            }
            ModuleType::None => {
                error!(
                    "[module] cannot load module '{}', file not exists.",
                    specifier
                );
                exit(-1);
            }
        };

        // transpile only applies for local code,
        // for remote code shall be pre-transpiled
        if let ModuleType::Local(script_type) = module_type {
            code = transpile(&code, script_type).unwrap().code;
            // print only the first 255 characters
            // debug!("code transpiled\n{}", &code[..(255.min(code.len()))]);
        }

        let resource_name = resolved_specifier.clone().into_v8(scope).into();
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
            .insert(script_id, resolved_specifier.clone());

        let global_module = Global::new(scope, module);

        module_info.script_id = Some(script_id);
        module_info.module = Some(global_module);

        // println!("?? {:?} {}", module.get_status(), resolved_specifier);

        true
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

    pub fn get_module_result(&self, resolved_specifier: &str) -> Option<Global<Value>> {
        let module_info = self
            .module_info_map
            .get(resolved_specifier)
            .expect(format!("cannot find module {}", resolved_specifier).as_str());

        module_info.result.clone()
    }
}
