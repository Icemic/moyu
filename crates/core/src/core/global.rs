use std::sync::Arc;

use doufu_pal::visible_hand::{InvisibleHand, VisibleHand};

use super::Core;

static mut CORE: InvisibleHand<Arc<Core>> = InvisibleHand::new();

#[inline]
pub fn get_core<'a>() -> &'a Arc<Core> {
    unsafe { CORE.get() }
}

#[inline]
pub fn set_core(core: Arc<Core>) -> VisibleHand<Arc<Core>> {
    unsafe {
        CORE.set(core).expect("Failed to set core instance.");
        CORE.intervent()
    }
}
