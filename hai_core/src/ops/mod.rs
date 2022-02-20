use std::{cell::RefCell, rc::Rc};

use hai_js_runtime::{
    bind_function,
    v8::{
        FunctionCallbackArguments, FunctionTemplate, HandleScope, Local, Object, ObjectTemplate,
        ReturnValue, String,
    },
    Shared,
};

use crate::state::State;

pub fn init(handle_scope: &mut HandleScope, global: &Local<Object>) {
    bind_function!(
      to global;
      of handle_scope;
      "testCommand" => test
    );
}

fn test(scope: &mut HandleScope, args: FunctionCallbackArguments, _: ReturnValue) {
    let shared = scope.get_slot::<Rc<RefCell<Shared>>>().unwrap();
    let shared = shared.borrow();

    let state = shared.state::<State>();
    let state = state.lock().unwrap();
    state.test();
}
