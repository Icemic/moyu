use std::{cell::RefCell, rc::Rc};
use v8::{
    Exception, Function, FunctionCallbackArguments, FunctionTemplate, Global, HandleScope, Local,
    Number, Object, ReturnValue, String,
};

use crate::{state::State, timer::TimerType, utils::IntoV8};

pub fn init(handle_scope: &mut HandleScope, global: &Local<Object>) {
    bind_function!(
      to global;
      of handle_scope;
      "setTimeout" => set_timeout,
      "setInterval" => set_interval,
      "clearTimeout" => clear_timeout_or_interval,
      "clearInterval" => clear_timeout_or_interval
    );
}

fn set_timeout(scope: &mut HandleScope, args: FunctionCallbackArguments, ret: ReturnValue) {
    create_timer(scope, args, ret, TimerType::Timeout);
}

fn set_interval(scope: &mut HandleScope, args: FunctionCallbackArguments, ret: ReturnValue) {
    create_timer(scope, args, ret, TimerType::Interval);
}

fn clear_timeout_or_interval(
    scope: &mut HandleScope,
    args: FunctionCallbackArguments,
    _: ReturnValue,
) {
    let handler_id = match Local::<Number>::try_from(args.get(0)) {
        Ok(handler_id) => handler_id,
        Err(err) => {
            let error_message: Local<String> = format!("{}", err).into_v8(scope);
            let error = Exception::error(scope, error_message);
            scope.throw_exception(error);
            return;
        }
    };

    let handler_id = handler_id.value() as i32;

    let timer = {
        let state = scope.get_slot::<Rc<RefCell<State>>>().unwrap();
        let state = state.borrow();
        state.timer()
    };
    let mut timer = timer.borrow_mut();
    timer.cancel_timer(handler_id);
}

fn create_timer(
    scope: &mut HandleScope,
    args: FunctionCallbackArguments,
    mut ret: ReturnValue,
    t: TimerType,
) {
    let callback = match Local::<Function>::try_from(args.get(0)) {
        Ok(func) => func,
        Err(err) => {
            let error_message: Local<String> = format!("{}", err).into_v8(scope);
            let error = Exception::error(scope, error_message);
            scope.throw_exception(error);
            return;
        }
    };
    let duration = match Local::<Number>::try_from(args.get(1)) {
        Ok(duration) => duration,
        Err(err) => {
            let error_message: Local<String> = format!("{}", err).into_v8(scope);
            let error = Exception::error(scope, error_message);
            scope.throw_exception(error);
            return;
        }
    };

    let mut duration = duration.value() as u64;

    // https://developer.mozilla.org/en-US/docs/Web/API/WindowOrWorkerGlobalScope/setTimeout#Minimum_delay_and_timeout_nesting
    if duration < 4 {
        duration = 4;
    }

    let callback = Global::new(scope, callback);

    let timer = {
        let state = scope.get_slot::<Rc<RefCell<State>>>().unwrap();
        let state = state.borrow();
        state.timer()
    };
    let mut timer = timer.borrow_mut();
    let handler_id = timer.add_timer(t, callback, duration);

    let handler_id = handler_id.into_v8(scope);
    ret.set(handler_id.into());
}
