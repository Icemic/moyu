#[cfg_attr(native, path = "fs/appdata_native.rs")]
#[cfg_attr(web, path = "fs/appdata_web.rs")]
mod appdata;
mod appdata_common;
mod open;
mod read;

pub use appdata::*;
pub use appdata_common::*;
pub use open::*;
pub use read::*;
