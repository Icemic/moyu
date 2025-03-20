#[cfg(any(desktop, web))]
mod events;
#[cfg(any(desktop, web))]
mod gamepad;
#[cfg(any(desktop, web))]
mod plugin;
#[cfg(any(desktop, web))]
mod utils;

#[cfg(any(desktop, web))]
pub use events::*;
#[cfg(any(desktop, web))]
pub use gamepad::*;
#[cfg(any(desktop, web))]
pub use plugin::*;
