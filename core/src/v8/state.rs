use futures::task::AtomicWaker;
use std::{cell::RefCell, rc::Rc};

use super::module::ModuleLoader;

pub struct State {
    pub module_loader: Rc<RefCell<ModuleLoader>>,
    pub waker: AtomicWaker,
}

impl State {
    pub fn new() -> Self {
        let mut module_loader = ModuleLoader::new();
        module_loader.setup_entry_module();
        State {
            module_loader: Rc::new(RefCell::new(module_loader)),
            waker: AtomicWaker::new(),
        }
    }

    pub fn module_loader(&self) -> Rc<RefCell<ModuleLoader>> {
        self.module_loader.clone()
    }
}
