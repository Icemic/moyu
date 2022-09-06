mod callbacks;
mod types;
mod utils;

use anyhow::Result;
pub use callbacks::*;
use hai_pal::url::{resolve_package_from, Url};
pub use types::*;
pub use utils::*;

use futures::stream::FuturesUnordered;
use futures::task::AtomicWaker;
use futures::{Future, FutureExt};
use log::debug;
use std::collections::HashMap;
use std::pin::Pin;
use v8::{
    script_compiler::{self, Source},
    Global, HandleScope, Local, Module as V8Module, ModuleRequest, ModuleStatus, ScriptOrigin,
    Value,
};

use crate::utils::IntoV8;

#[derive(Debug, Default)]
pub struct ModuleLoader {
    pub resolved_names: HashMap<i32, Url>,
    pub modules: HashMap<Url, Module>,
    pub pending: FuturesUnordered<
        Pin<Box<dyn Future<Output = (Url, Result<std::string::String, anyhow::Error>)>>>,
    >,
    // pub pending_modules: Vec<ModulePending>,
    pub waker: AtomicWaker,
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_resolved_file_path_from_script_id(&self, script_id: i32) -> Option<&Url> {
        self.resolved_names.get(&script_id)
    }

    pub fn push_module_loading_task(&mut self, base_dir: &Url, specifier: String) -> Result<Url> {
        // resolve to absolute referrer path
        let module = match resolve_package_from(specifier.as_str(), base_dir.clone()) {
            Ok(resolved_file_path) => Module {
                specifier: specifier.clone(),
                resolved_file_path,
                module_type: ModuleType::Local,
                script_id: None,
                module: None,
                result: None,
            },
            Err(err) => {
                return Err(anyhow::format_err!(
                    "Cannot find module '{}': {}",
                    specifier,
                    err.to_string()
                ));
            }
        };

        debug!("resolved module: {:?}", module);

        let resolved_file_path = module.resolved_file_path.clone();
        let module_type = module.module_type.clone();

        let load_fn = async move {
            // TODO: async load code, create module, then modify

            let code = match module_type {
                ModuleType::Local => {
                    let code = read_code_local(&resolved_file_path).await;
                    Ok(code)
                }
                ModuleType::Remote => {
                    let code = read_code_remote(&resolved_file_path).await;
                    Ok(code)
                }
            };

            (resolved_file_path, code)
        }
        .boxed_local();

        let resolved_file_path = module.resolved_file_path.clone();

        self.modules.insert(resolved_file_path.clone(), module);
        self.pending.push(load_fn);

        // activate poll
        self.waker.wake();

        Ok(resolved_file_path)
    }

    pub fn compile_module(
        &mut self,
        scope: &mut HandleScope,
        resolved_file_path: &Url,
        code: &str,
    ) -> Result<()> {
        let module = self
            .modules
            .get_mut(resolved_file_path)
            .expect(format!("cannot find module '{}'", resolved_file_path.as_str()).as_str());

        let resource_name = resolved_file_path.as_str().into_v8(scope).into();
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
        let v8module = script_compiler::compile_module(scope, source).unwrap();

        // save to state
        let script_id = v8module.script_id().unwrap();

        self.resolved_names
            .insert(script_id, resolved_file_path.clone());

        let global_module = Global::new(scope, v8module);

        module.script_id = Some(script_id);
        module.module = Some(global_module);

        // pend dependencies' imports
        let module_requests = v8module.get_module_requests();
        let current_dir = module.resolved_file_path.join("./").unwrap();
        for i in 0..module_requests.length() {
            let module_request: Local<ModuleRequest> =
                module_requests.get(scope, i).unwrap().try_into().unwrap();
            let specifier = module_request.get_specifier().to_rust_string_lossy(scope);
            self.push_module_loading_task(&current_dir, specifier)?;
        }

        Ok(())
    }

    pub fn instantiate_module<'a>(
        scope: &mut HandleScope,
        module: Global<V8Module>,
        resolved_file_path: &Url,
    ) -> bool {
        let module = Local::new(scope, module);

        if module.get_status() != ModuleStatus::Uninstantiated {
            println!(
                "???? {:?} {}",
                module.get_status(),
                resolved_file_path.as_str()
            );
            return false;
            // unreachable!(
            //     "cannot instantiate a module '{}' which has been instantiated or errored",
            //     resolved_file_path
            // );
        }

        debug!("instantiate module '{}'", resolved_file_path.as_str());

        // instantiate and run module code
        module
            .instantiate_module(scope, module_resolve_callback)
            .unwrap();

        true
    }

    pub fn evaluate_module(
        scope: &mut HandleScope,
        module: Global<V8Module>,
        resolved_file_path: &Url,
    ) -> Option<Global<Value>> {
        let module = Local::new(scope, module);

        if module.get_status() == ModuleStatus::Evaluated {
            return None;
        }

        if module.get_status() != ModuleStatus::Instantiated {
            return None;
        }

        debug!("evaluate module '{}'", resolved_file_path.as_str());

        if let Some(result) = module.evaluate(scope) {
            debug!(
                "instantiate module '{}' finished",
                resolved_file_path.as_str()
            );

            // TODO: error handling?

            // result must be a promise by design
            let result = Global::new(scope, result);

            return Some(result);
        }

        unreachable!("cannot evaluate a module which does not exist.");
    }

    pub fn get_module(&self, resolved_file_path: &Url) -> Option<Global<V8Module>> {
        let module = self
            .modules
            .get(resolved_file_path)
            .expect(format!("cannot find module {}", resolved_file_path.as_str()).as_str());

        module.module.clone()
    }
}
