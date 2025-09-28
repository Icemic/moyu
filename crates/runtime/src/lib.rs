mod console;
mod module;
mod vm;

use std::sync::Arc;

use moyu_pal::visible_hand::{InvisibleHand, VisibleHand};
pub use vm::QuickVM;

static JSVM: InvisibleHand<Arc<QuickVM>> = InvisibleHand::new();

pub fn setup_vm() -> VisibleHand<Arc<QuickVM>> {
    let vm = Arc::new(QuickVM::new());
    JSVM.set(vm.clone()).ok();
    JSVM.intervent()
}

pub fn get_vm<'a>() -> &'a Arc<QuickVM> {
    JSVM.get()
}

pub fn try_get_vm<'a>() -> Option<&'a Arc<QuickVM>> {
    JSVM.try_get()
}

pub mod quickjs_rusty {
    pub use quickjs_rusty::*;
}
