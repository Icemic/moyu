use futures::task::AtomicWaker;
use std::{cell::RefCell, rc::Rc};

use super::{module::ModuleLoader, timer::TimerScheduler};

pub struct Shared {
    pub module_loader: Rc<RefCell<ModuleLoader>>,
    pub timer: Rc<RefCell<TimerScheduler>>,
    pub waker: AtomicWaker,
}

impl Shared {
    pub fn new() -> Self {
        let module_loader = ModuleLoader::new();
        let timer = TimerScheduler::new();

        Shared {
            module_loader: Rc::new(RefCell::new(module_loader)),
            timer: Rc::new(RefCell::new(timer)),
            waker: AtomicWaker::new(),
        }
    }

    pub fn module_loader(&self) -> Rc<RefCell<ModuleLoader>> {
        self.module_loader.clone()
    }

    pub fn timer(&self) -> Rc<RefCell<TimerScheduler>> {
        self.timer.clone()
    }
}
