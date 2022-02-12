#[macro_use]
mod macros;
mod internals;
mod module;
mod state;
mod utils;

use std::{
    cell::RefCell,
    rc::Rc,
    task::{Context as TaskContext, Poll},
};
use tokio::macros::support::poll_fn;
use v8::{Context, ContextScope, Global, HandleScope, Isolate, Local, OwnedIsolate};

use crate::v8::{
    module::{ModuleLoader, ModulePendingStatus},
    state::State,
};
use module::dynamic_import_callback;

pub struct JSRuntime {
    isolate: OwnedIsolate,
    global_context: Global<Context>,
}

impl JSRuntime {
    pub fn new() -> Self {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();

        let mut isolate = Isolate::new(Default::default());
        isolate.set_host_import_module_dynamically_callback(dynamic_import_callback);
        // isolate.set_capture_stack_trace_for_uncaught_exceptions(true, 10);
        // isolate.set_promise_reject_callback();
        // isolate.set_host_initialize_import_meta_object_callback();

        let (global_context, state) = {
            let scope = &mut HandleScope::new(&mut isolate);
            let context = Context::new(scope);

            {
                // inject internal apis
                let context_scope = &mut ContextScope::new(scope, context);
                let global = context.global(context_scope);
                internals::setup(context_scope, &global);
            }

            // save global context
            let global_context = Global::new(scope, context);

            // save state
            let state = State::new();

            (global_context, state)
        };

        // save state to slot
        isolate.set_slot(Rc::new(RefCell::new(state)));

        Self {
            isolate,
            global_context,
        }
    }

    pub fn get_global_context(&mut self) -> v8::Global<v8::Context> {
        self.global_context.clone()
    }

    pub fn get_isolate_mut(&mut self) -> &mut v8::OwnedIsolate {
        &mut self.isolate
    }

    pub fn get_handle_scope(&mut self) -> v8::HandleScope {
        let context = self.get_global_context();
        v8::HandleScope::with_context(self.get_isolate_mut(), context)
    }

    pub async fn run_event_loop(&mut self) {
        poll_fn(|cx| self.poll_event_loop(cx)).await
    }

    fn poll_event_loop(&mut self, cx: &mut TaskContext<'_>) -> Poll<()> {
        let state = self.isolate.get_slot::<Rc<RefCell<State>>>().unwrap();
        let state = state.borrow_mut();

        // register waker to isolate state
        state.waker.register(cx.waker());

        let module_loader = state.module_loader();

        drop(state);

        loop {
            // borrow module loader
            // be careful that module_loader should not have any borrow when `instantiate_module` executes,
            // or it will panic.
            let mut module_loader_mut = module_loader.borrow_mut();
            let module_pending = module_loader_mut.pending_modules.pop();

            if let Some(mut module_pending) = module_pending {
                let resolved_specifier = module_pending.resolved_specifier.clone();
                let promise_resolver = &module_pending.promise_resolver;

                // get a new handle scope for instantiate module and evaluate it
                let scope = &mut self.get_handle_scope();

                match module_pending.status {
                    ModulePendingStatus::Created => {
                        module_pending.status = ModulePendingStatus::Resolved;
                        module_loader_mut.pending_modules.push(module_pending);
                        module_loader_mut.resolve_module(scope, &resolved_specifier);
                    }
                    ModulePendingStatus::Resolved => {
                        let module = module_loader_mut.get_module(&resolved_specifier).unwrap();

                        module_pending.status = ModulePendingStatus::Instantiated;
                        module_loader_mut.pending_modules.push(module_pending);

                        drop(module_loader_mut);

                        ModuleLoader::instantiate_module(scope, module, &resolved_specifier);
                    }
                    ModulePendingStatus::Instantiated => {
                        let module = module_loader_mut.get_module(&resolved_specifier).unwrap();

                        module_pending.status = ModulePendingStatus::Evaluated;
                        module_loader_mut.pending_modules.push(module_pending);

                        drop(module_loader_mut);

                        let result =
                            ModuleLoader::evaluate_module(scope, module, &resolved_specifier);

                        let mut module_loader_mut = module_loader.borrow_mut();

                        let mut module_info = module_loader_mut
                            .module_info_map
                            .get_mut(&resolved_specifier)
                            .unwrap();
                        if let Some(result) = result {
                            module_info.result = Some(result);
                        }
                    }
                    ModulePendingStatus::Evaluated => {
                        if let Some(promise_resolver) = promise_resolver {
                            let result = module_loader_mut
                                .get_module_result(&resolved_specifier)
                                .unwrap();
                            let result = Local::new(scope, result);

                            let promise_resolver = Local::new(scope, promise_resolver);
                            promise_resolver.resolve(scope, result);
                        }
                    }
                };
            } else {
                cx.waker().clone().wake();
                break;
            }
        }

        Poll::Pending
    }
}
