use std::ffi::c_void;
use std::mem::forget;
use std::sync::{Arc, Mutex};

use once_cell::sync::OnceCell;

use crate::state::State;

static STATE: OnceCell<usize> = OnceCell::new();

pub fn get_shared_state() -> Arc<Mutex<State>> {
    let p = *STATE.get().unwrap() as *const c_void;
    let ptr = p as *const Mutex<State>;
    let r = unsafe { Arc::from_raw(ptr) };
    let r_cloned = r.clone();

    // keep ptr leaked
    forget(r);

    r_cloned
}

pub fn set_shared_state(state: Arc<Mutex<State>>) {
    let p = Arc::into_raw(state) as *const c_void as usize;
    STATE.set(p);
}
