use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use anyhow::Result;
use log::debug;

/// A holder for global variables that release resources when dropped.
pub struct InvisibleHand<T> {
    data: AtomicPtr<T>,
}

impl<T> Default for InvisibleHand<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> InvisibleHand<T> {
    pub const fn new() -> Self {
        Self {
            data: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub fn set(&self, value: T) -> Result<()> {
        let boxed = Box::into_raw(Box::new(value));
        match self.data.compare_exchange(
            ptr::null_mut(),
            boxed,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => Ok(()),
            Err(_) => {
                // Failed to set, clean up the allocated memory
                unsafe { drop(Box::from_raw(boxed)) };
                Err(anyhow::anyhow!("Resource already initialized"))
            }
        }
    }

    pub fn get(&self) -> &T {
        let ptr = self.data.load(Ordering::Acquire);
        if ptr.is_null() {
            panic!("Resource not initialized");
        }
        unsafe { &*ptr }
    }

    pub fn try_get(&self) -> Option<&T> {
        let ptr = self.data.load(Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { &*ptr })
        }
    }

    pub fn intervent(&'static self) -> VisibleHand<T> {
        VisibleHand { hand: self }
    }
}

pub struct VisibleHand<T: 'static> {
    hand: &'static InvisibleHand<T>,
}

impl<T> Drop for VisibleHand<T> {
    fn drop(&mut self) {
        debug!(
            "dropping global variable of type: {}",
            std::any::type_name::<T>()
        );
        let ptr = self.hand.data.swap(ptr::null_mut(), Ordering::AcqRel);
        if !ptr.is_null() {
            unsafe { drop(Box::from_raw(ptr)) };
        }
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

    static _GLOBAL_HAND: InvisibleHand<i32> = InvisibleHand::new();

    #[test]
    fn test_visible_hand() {
        _GLOBAL_HAND.set(1).unwrap();
        let hand = _GLOBAL_HAND.intervent();
        assert!(_GLOBAL_HAND.try_get().is_some());
        assert_eq!(*_GLOBAL_HAND.get(), 1);
        drop(hand);
        assert!(_GLOBAL_HAND.try_get().is_none());
    }
}
