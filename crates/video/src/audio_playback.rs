pub mod clock;
pub mod ring_buffer;

#[cfg(native)]
pub mod native;

#[cfg(web)]
pub mod web;
