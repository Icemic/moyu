#![cfg(not(target_arch = "wasm32"))]

mod console;
mod module;
mod ops;
mod vm;

use std::sync::Arc;
use std::sync::OnceLock;

use moyu_pal::visible_hand::{InvisibleHand, VisibleHand};
pub use vm::QuickVM;

static JSVM: InvisibleHand<Arc<QuickVM>> = InvisibleHand::new();
type VmWakeHook = Arc<dyn Fn() + Send + Sync + 'static>;
static VM_WAKE_HOOK: OnceLock<VmWakeHook> = OnceLock::new();

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

pub fn set_vm_wake_hook(hook: VmWakeHook) {
    let _ = VM_WAKE_HOOK.set(hook);
}

pub(crate) fn invoke_vm_wake_hook() {
    if let Some(hook) = VM_WAKE_HOOK.get() {
        hook();
    }
}

pub mod quickjs_rusty {
    pub use quickjs_rusty::*;
}
