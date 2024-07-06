#[cfg(not(feature = "web"))]
pub use std::time::*;

#[cfg(feature = "web")]
pub use web_time::*;
