mod console;
mod timer;

use v8::{HandleScope, Local, Object};

pub fn setup(scope: &mut HandleScope, global: &Local<Object>) {
    console::init(scope, global);
    timer::init(scope, global);
}
