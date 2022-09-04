use log::error;
use std::{cell::RefCell, rc::Rc};
use v8::{
    CallbackScope, Context, FixedArray, Local, Module, Promise, PromiseRejectMessage, String, Value,
};

use super::utils::resolve_module_specifier;
use crate::shared::Shared;

/// load a module synchronously
/// used directly by module.instantiate_module which needs a v8::Module instance
/// while the instance must created synchronously, its instantiation and
pub fn module_resolve_callback<'a>(
    context: Local<'a, Context>,
    specifier: Local<'a, String>,
    _import_assertions: Local<'a, FixedArray>,
    referrer: Local<'a, Module>,
) -> Option<Local<'a, Module>> {
    let scope = &mut unsafe { CallbackScope::new(context) };

    let shared = scope.get_slot_mut::<Rc<RefCell<Shared>>>().unwrap();
    let shared = shared.borrow();
    let module_loader = shared.module_loader();

    drop(shared);

    let specifier = specifier.to_rust_string_lossy(scope);
    let module_loader = module_loader.borrow_mut();
    let referrer_name = module_loader
        .get_resolved_specifier_from_script_id(referrer.script_id().unwrap())
        .unwrap();

    let (_, resolved_specifier) = resolve_module_specifier(&specifier, &referrer_name);

    let module = module_loader.get_module(&resolved_specifier).unwrap();
    let module = Local::new(scope, module);

    Some(module)
}

pub extern "C" fn dynamic_import_callback(
    _context: Local<Context>,
    _host_defined_options: v8::Local<v8::Data>,
    _resource_name: Local<Value>,
    _specifier: Local<String>,
    _import_assertions: Local<FixedArray>,
) -> *mut Promise {
    // let scope = &mut unsafe { CallbackScope::new(context) };
    // let shared = scope.get_slot_mut::<Rc<RefCell<shared>>>().unwrap();
    // let shared = shared.borrow();
    // let module_loader = shared.module_loader();
    // let module_loader_mut = module_loader.borrow_mut();

    // drop(shared);
    // drop(module_loader_mut);

    // let referrer_name = referrer.get_resource_name().to_rust_string_lossy(scope);
    // let specifier = specifier.to_rust_string_lossy(scope);

    // // create promise resolver
    // let resolver = PromiseResolver::new(scope).unwrap();
    // let promise: *mut Promise = &*resolver.get_promise(scope) as *const _ as *mut _;
    // let resolver = Global::new(scope, resolver);

    // let mut module_loader_mut = module_loader.borrow_mut();

    // let (_, resolved_specifier) = resolve_module_specifier(&specifier, &referrer_name);

    // let module = module_loader_mut.get_module(&resolved_specifier).unwrap();
    // // let module = Local::new(scope, module);

    // let result = ModuleLoader::evaluate_module(scope, module, &resolved_specifier).unwrap();

    // add to queue
    // let resolved_specifier = module_loader_mut.create_module_info(referrer_name, specifier);
    // module_loader_mut.enqueue_module_pending(
    //     ModulePendingStatus::Created,
    //     &resolved_specifier,
    //     Some(resolver),
    // );

    // promise

    todo!("not supported yet");
}

pub extern "C" fn promise_reject_callback(msg: PromiseRejectMessage) {
    // @see https://github.com/denoland/deno/blob/307d84cfa5c1489ddfc8477f6561676356399e8c/core/bindings.rs#L409
    // let scope = &mut unsafe { v8::CallbackScope::new(&msg) };

    let event = msg.get_event();
    error!("Uncaught promise reject event: {:?}", event);
}
