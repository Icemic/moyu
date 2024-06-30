use std::sync::OnceLock;

use anyhow::Result;
use log::debug;

/// A holder for global variables that release resources when dropped.
pub struct InvisibleHand<T> {
    once_lock: OnceLock<T>,
}

impl<T> InvisibleHand<T> {
    pub const fn new() -> Self {
        Self {
            once_lock: OnceLock::new(),
        }
    }

    pub fn set(&self, value: T) -> Result<()> {
        self.once_lock
            .set(value)
            .map_err(|_| anyhow::anyhow!("Resource already initialized"))?;
        Ok(())
    }

    pub fn get(&self) -> &T {
        self.once_lock.get().expect("Resource not initialized")
    }

    pub fn try_get(&self) -> Option<&T> {
        self.once_lock.get()
    }

    pub fn intervent(&'static mut self) -> VisibleHand<T> {
        VisibleHand {
            once_lock: &mut self.once_lock,
        }
    }
}

pub struct VisibleHand<T: 'static> {
    once_lock: &'static mut OnceLock<T>,
}

impl<T> Drop for VisibleHand<T> {
    fn drop(&mut self) {
        debug!(
            "dropping global variable of type: {}",
            std::any::type_name::<T>()
        );
        let _ = self.once_lock.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invisible_hand() {
        let hand = InvisibleHand::new();
        hand.set(1).unwrap();
        assert_eq!(*hand.get(), 1);
    }

    static mut _GLOBAL_HAND: InvisibleHand<i32> = InvisibleHand::new();

    #[test]
    fn test_visible_hand() {
        unsafe {
            _GLOBAL_HAND.set(1).unwrap();
            let hand = _GLOBAL_HAND.intervent();
            assert_eq!(_GLOBAL_HAND.once_lock.get().is_some(), true);
            drop(hand);
            assert_eq!(_GLOBAL_HAND.once_lock.get().is_some(), false);
        }
    }
}
