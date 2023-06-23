use std::ptr::null_mut;

use hai_pal::env::entry_dir;
use quickjspp::{Context, ExecutionError};

use crate::console::log_handler;
use crate::module::{module_loader, module_normalize};

pub struct QuickVM {
    context: Context,
}

unsafe impl Send for QuickVM {}
unsafe impl Sync for QuickVM {}

impl QuickVM {
    pub fn new() -> Self {
        let context = Context::builder()
            .console(log_handler)
            .build()
            .expect("Failed to create QuickJS context");

        context.set_module_loader(
            Box::new(module_loader),
            Some(Box::new(module_normalize)),
            null_mut(),
        );

        Self { context }
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn prepare_entry(&self) -> Result<(), ExecutionError> {
        let module_name = {
            let entry_dir = entry_dir();

            if entry_dir.to_string().ends_with("/") {
                "./index".to_string()
            } else {
                let specifier = entry_dir.path_segments().unwrap().last().unwrap();
                format!("./{}", specifier)
            }
        };

        self.context.run_module(&module_name)
    }
}
