mod custom;
mod focus;
mod fullscreen;
mod keyboard;
mod mouse;
mod node;
mod raf;
mod resize;
mod touch;
mod wheel;

pub use custom::*;
pub use focus::*;
pub use fullscreen::*;
pub use keyboard::*;
pub use mouse::*;
pub use node::*;
pub use raf::*;
pub use resize::*;
pub use touch::*;
pub use wheel::*;

use serde::{Deserialize, Serialize};

use crate::traits::Event;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaiEvent<'a, T: Event> {
    pub name: &'a str,
    // `body` may be a Map instead of a Object after serialization to JS,
    // which due to a known issue in serde-wasm-bindgen and serde.
    // See https://github.com/RReverser/serde-wasm-bindgen/issues/9
    //
    // For compatibility, we have to destruct `body` in JS side to make it an Object.
    pub body: T,
}

impl<'a, T: Event> HaiEvent<'a, T> {
    pub fn from_event(body: T) -> Self {
        Self {
            name: body.name(),
            body,
        }
    }
}
