mod audio;
mod kira_static_data;
mod manager;
mod utils;
mod wildcard;

#[cfg(target_arch = "wasm32")]
mod web_backend;

#[cfg(not(target_arch = "wasm32"))]
type AudioBackend = kira::DefaultBackend;
#[cfg(target_arch = "wasm32")]
type AudioBackend = web_backend::WebAudioBackend;

pub use audio::Audio;
pub use manager::AudioManager;
