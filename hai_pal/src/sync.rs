#[cfg(not(target_arch = "wasm32"))]
pub use parking_lot::*;
#[cfg(target_arch = "wasm32")]
pub use std::sync::{
    LockResult, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError,
    TryLockResult,
};
