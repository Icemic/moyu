use futures::task::AtomicWaker;
use hai_pal::sync::RwLock;
use std::{cell::RefCell, ffi::c_void, mem::forget, rc::Rc, sync::Arc};

use super::{module::ModuleLoader, timer::TimerScheduler};

pub struct Shared {
    pub module_loader: Rc<RefCell<ModuleLoader>>,
    pub scheduler: Rc<RefCell<TimerScheduler>>,
    pub waker: AtomicWaker,
    /// external state binding to js runtime
    /// which makes injected methods read or write custom data
    pub state: *const c_void,
}

impl Shared {
    pub fn new<T>(state: Arc<RwLock<T>>) -> Self {
        let module_loader = ModuleLoader::new();
        let timer = TimerScheduler::new();

        let state = Arc::into_raw(state) as *const c_void;

        Shared {
            module_loader: Rc::new(RefCell::new(module_loader)),
            scheduler: Rc::new(RefCell::new(timer)),
            waker: AtomicWaker::new(),
            state,
        }
    }

    pub fn module_loader(&self) -> Rc<RefCell<ModuleLoader>> {
        self.module_loader.clone()
    }

    pub fn scheduler(&self) -> Rc<RefCell<TimerScheduler>> {
        self.scheduler.clone()
    }

    pub fn state<T>(&self) -> Arc<RwLock<T>> {
        let ptr = self.state as *const RwLock<T>;
        let r = unsafe { Arc::from_raw(ptr) };
        let r_cloned = r.clone();

        // keep ptr leaked
        forget(r);

        r_cloned
    }
}
