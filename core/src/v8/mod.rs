#[macro_use]
mod macros;
mod internals;
mod module;
mod state;
mod utils;

use log::info;
use std::{env, rc::Rc, cell::RefCell};
use v8::{Context, ContextScope, HandleScope, Isolate, OwnedIsolate};

use internals::setup;
use module::dynamic_import_callback;

use self::state::State;
use crate::v8::module::import_module;

pub struct V8 {
    isolate: OwnedIsolate,
}

impl V8 {
    pub fn init() -> Self {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();

        let mut isolate = Isolate::new(Default::default());

        let state = State::new();

        isolate.set_slot(Rc::new(RefCell::new(state)));
        isolate.set_host_import_module_dynamically_callback(dynamic_import_callback);

        V8 { isolate }
    }

    pub fn run(&mut self) {
        let mut entry_dir = env::var("HAI_ENTRY")
            .unwrap_or(env::current_dir().unwrap().to_str().unwrap().to_string());

        info!("[module] entry '{}'", entry_dir);

        let scope = &mut HandleScope::new(&mut self.isolate);
        let global_context = Context::new(scope);
        let scope = &mut ContextScope::new(scope, global_context);

        // install internal objects to global
        let global = global_context.global(scope);
        setup(scope, &global);

        // input shall be a referrer name but entry_dir is a directory, so do some hack
        entry_dir.push_str("./index");
        // start from entry file
        import_module(scope, Some(entry_dir), None, "./index".to_string(), None);
    }
}
