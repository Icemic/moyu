#[cfg(native)]
pub use std::time::*;
#[cfg(native)]
pub use tokio::time::{
    interval, interval_at, sleep, sleep_until, timeout, timeout_at, Interval, Sleep, Timeout,
};

#[cfg(web)]
pub use wasmtimer::std::*;
#[cfg(web)]
pub use wasmtimer::tokio::{
    interval, interval_at, sleep, sleep_until, timeout, timeout_at, Interval, Sleep, Timeout,
};
