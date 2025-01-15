#[cfg(native)]
pub use std::time::*;

#[cfg(web)]
pub use web_time::*;
