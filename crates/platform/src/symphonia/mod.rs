mod opus_decoder;

use lazy_static::lazy_static;
use symphonia::core::codecs::CodecRegistry;
use symphonia::default::register_enabled_codecs;

use opus_decoder::OpusDecoder;

lazy_static! {
    static ref CUSTOM_CODEC_REGISTRY: CodecRegistry = {
        let mut registry = CodecRegistry::new();
        register_enabled_codecs(&mut registry);
        registry.register_all::<OpusDecoder>();

        registry
    };
}

/// Returns the shared codec registry with Opus support.
pub fn get_codec() -> &'static CodecRegistry {
    &CUSTOM_CODEC_REGISTRY
}

/// Returns the default probe for container format detection.
pub fn get_probe() -> &'static symphonia::core::probe::Probe {
    symphonia::default::get_probe()
}
