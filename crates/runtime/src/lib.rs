mod console;
mod module;
mod vm;

use std::sync::Arc;

use hai_pal::visible_hand::{InvisibleHand, VisibleHand};
pub use vm::QuickVM;

static mut JSVM: InvisibleHand<Arc<QuickVM>> = InvisibleHand::new();

pub fn setup_vm() -> VisibleHand<Arc<QuickVM>> {
    let vm = Arc::new(QuickVM::new());
    unsafe {
        JSVM.set(vm.clone()).ok();
        JSVM.intervent()
    }
}

pub fn get_vm<'a>() -> &'a Arc<QuickVM> {
    unsafe { JSVM.get() }
}

pub mod quickjs_rusty {
    pub use quickjs_rusty::*;
}
