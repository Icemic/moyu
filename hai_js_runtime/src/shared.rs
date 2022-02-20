use futures::task::AtomicWaker;
use std::{
    cell::RefCell,
    ffi::c_void,
    rc::Rc,
    sync::{Arc, Mutex},
};

use super::{module::ModuleLoader, timer::TimerScheduler};

pub struct Shared {
    pub module_loader: Rc<RefCell<ModuleLoader>>,
    pub timer: Rc<RefCell<TimerScheduler>>,
    pub waker: AtomicWaker,
    /// external state binding to js runtime
    /// which makes injected methods read or write custom data
    pub state: *const c_void,
}

impl Shared {
    pub fn new<T>(state: Arc<Mutex<T>>) -> Self {
        let module_loader = ModuleLoader::new();
        let timer = TimerScheduler::new();

        let state = Arc::into_raw(state) as *const c_void;

        Shared {
            module_loader: Rc::new(RefCell::new(module_loader)),
            timer: Rc::new(RefCell::new(timer)),
            waker: AtomicWaker::new(),
            state,
        }
    }

    pub fn module_loader(&self) -> Rc<RefCell<ModuleLoader>> {
        self.module_loader.clone()
    }

    pub fn timer(&self) -> Rc<RefCell<TimerScheduler>> {
        self.timer.clone()
    }

    pub fn state<T>(&self) -> Arc<Mutex<T>> {
        // let boxed = unsafe { transmute::<*mut c_void, Box<T>>(self.state) };
        // boxed.as_ref().clone()
        let ptr = self.state as *const Mutex<T>;
        unsafe { Arc::from_raw(ptr) }
    }
}
