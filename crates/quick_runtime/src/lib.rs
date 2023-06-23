mod console;
mod module;
mod vm;

use std::sync::{Arc, OnceLock};

pub use vm::QuickVM;

static VM_INSTANCE: OnceLock<Arc<QuickVM>> = OnceLock::new();

pub fn setup_vm() -> Arc<QuickVM> {
    let vm = Arc::new(QuickVM::new());

    VM_INSTANCE.set(vm.clone()).ok();

    vm
}

pub fn get_vm<'a>() -> &'a Arc<QuickVM> {
    VM_INSTANCE.get().expect("VM not initialized")
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
