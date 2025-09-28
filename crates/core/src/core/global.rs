use std::sync::Arc;

use moyu_pal::visible_hand::{InvisibleHand, VisibleHand};

use super::Core;

static CORE: InvisibleHand<Arc<Core>> = InvisibleHand::new();

#[inline]
pub fn get_core<'a>() -> &'a Arc<Core> {
    CORE.get()
}

#[inline]
pub fn set_core(core: Arc<Core>) -> VisibleHand<Arc<Core>> {
    CORE.set(core).expect("Failed to set core instance.");
    CORE.intervent()
}
