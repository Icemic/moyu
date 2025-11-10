#[cfg(native)]
mod native;
#[cfg(native)]
pub use native::*;

#[cfg(web)]
mod web;
#[cfg(web)]
pub use web::*;
