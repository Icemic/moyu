#[macro_use]
mod macros;
mod internals;
mod module;
mod shared;
mod timer;

pub mod prelude;
pub mod utils;

use futures::{future::poll_fn, StreamExt};
use log::{error, info};
pub use shared::Shared;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
    task::{Context as TaskContext, Poll},
};
pub use v8;
use v8::{Context, ContextScope, Global, HandleScope, Isolate, Local, Object, OwnedIsolate, Value};

use self::module::{dynamic_import_callback, ModuleLoader};

pub struct JSRuntime {
    isolate: OwnedIsolate,
    global_context: Global<Context>,
}

impl JSRuntime {
    pub fn new<T>(state: Arc<Mutex<T>>) -> Self {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();

        let mut isolate = Isolate::new(Default::default());
        isolate.set_host_import_module_dynamically_callback(dynamic_import_callback);
        isolate.set_capture_stack_trace_for_uncaught_exceptions(true, 10);
        // isolate.set_promise_reject_callback();
        // isolate.set_host_initialize_import_meta_object_callback();

        let (global_context, shared) = {
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

            // save shared object
            let shared = Shared::new(state);

            (global_context, shared)
        };

        // save state to slot
        isolate.set_slot(Rc::new(RefCell::new(shared)));

        Self {
            isolate,
            global_context,
        }
    }

    pub fn with_global<T, K>(&mut self, mut callback: T) -> K
    where
        T: FnMut(&mut HandleScope, &Local<Object>) -> K,
    {
        let scope = &mut HandleScope::new(&mut self.isolate);
        let context = Local::new(scope, self.global_context.clone());
        let context_scope = &mut ContextScope::new(scope, context);
        let global = context.global(context_scope);
        callback(context_scope, &global)
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

    pub fn start(&mut self) {
        let module_loader = {
            let state = self.isolate.get_slot::<Rc<RefCell<Shared>>>().unwrap();
            let state = state.borrow_mut();
            state.module_loader()
        };

        let module_loader_mut = module_loader.borrow_mut();

        let resolved_specifier = module_loader_mut.entry_resolved_specifier.clone().unwrap();
        let module = module_loader_mut.get_module(&resolved_specifier).unwrap();

        drop(module_loader_mut);

        let scope = &mut self.get_handle_scope();
        ModuleLoader::instantiate_module(scope, module.clone(), &resolved_specifier);
        ModuleLoader::evaluate_module(scope, module, &resolved_specifier);
    }

    pub async fn prepare_static_modules(&mut self) {
        {
            let state = self.isolate.get_slot::<Rc<RefCell<Shared>>>().unwrap();
            let state = state.borrow_mut();
            let module_loader = state.module_loader();
            let mut module_loader = module_loader.borrow_mut();
            module_loader.prepare_from_entry();
        }
        poll_fn(|cx| self.poll_prepare_module(cx)).await
    }

    fn poll_prepare_module(&mut self, cx: &mut TaskContext<'_>) -> Poll<()> {
        let state = self.isolate.get_slot::<Rc<RefCell<Shared>>>().unwrap();
        let state = state.borrow_mut();

        // register waker to isolate state
        state.waker.register(cx.waker());

        let module_loader = state.module_loader();

        drop(state);

        let mut module_loader = module_loader.borrow_mut();

        // register waker to module loader
        module_loader.waker.register(cx.waker());

        match module_loader.pending.poll_next_unpin(cx) {
            Poll::Ready(None) => return Poll::Ready(()),
            Poll::Ready(Some((resolved_specifier, code))) => {
                let module_info = module_loader
                    .module_info_map
                    .get(&resolved_specifier)
                    .unwrap();
                if let Ok(code) = code {
                    info!(
                        "module '{}' loaded from '{}'",
                        module_info.specifier, resolved_specifier
                    );
                    let scope = &mut self.get_handle_scope();
                    module_loader.compile_module(scope, &resolved_specifier, &code);
                } else {
                    error!(
                        "cannot load module '{}', file '{}' not exists.",
                        module_info.specifier, resolved_specifier
                    );
                }
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_timers(&mut self, cx: &mut TaskContext<'_>) -> Poll<()> {
        let state = self.isolate.get_slot::<Rc<RefCell<Shared>>>().unwrap();
        let state = state.borrow_mut();

        // register waker to isolate state
        state.waker.register(cx.waker());

        let timer = state.timer();

        drop(state);

        let mut timer = timer.borrow_mut();

        // register waker to module loader
        timer.waker.register(cx.waker());

        match timer.pending.poll_next_unpin(cx) {
            Poll::Ready(None) => return Poll::Ready(()),
            Poll::Ready(Some(handler_id)) => {
                let callback = timer.consume_callback(handler_id);

                // timer should be dropped before callback was called,
                // or it may cause `already borrowed` error
                // when two or more callbacks (aka two or more setTimeout or setInterval) nested.
                drop(timer);

                if let Some(callback) = callback {
                    let scope = &mut self.get_handle_scope();
                    let context = Context::new(scope);
                    let global = context.global(scope);
                    let callback = Local::new(scope, callback);
                    // TODO: support pass extra arguments
                    let args: [Local<'_, Value>; 0] = [];
                    callback.call(scope, global.into(), &args);
                }

                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Pending => Poll::Pending,
        }
    }

    pub fn poll_tick(&mut self, cx: &mut TaskContext<'_>) -> Poll<()> {
        let prepare_module_result = self.poll_prepare_module(cx);
        let timers_result = self.poll_timers(cx);

        if prepare_module_result.is_ready() && timers_result.is_ready() {
            return Poll::Ready(());
        }
        Poll::Pending
    }

    pub async fn run_event_loop(&mut self) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            poll_fn(|cx| self.poll_tick(cx)).await;
        }
    }
}
