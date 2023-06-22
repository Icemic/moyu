use quickjspp::Context;

use crate::console::log_handler;

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

        Self { context }
    }
    pub fn context(&self) -> &Context {
        &self.context
    }
}
