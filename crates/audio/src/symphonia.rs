use lazy_static::lazy_static;
use symphonia::default::register_enabled_codecs;
use symphonia_core::codecs::CodecRegistry;

use crate::opus_decoder::OpusDecoder;

lazy_static! {
    static ref CUSTOM_CODEC_REGISTRY: CodecRegistry = {
        let mut registry = CodecRegistry::new();
        register_enabled_codecs(&mut registry);
        registry.register_all::<OpusDecoder>();

        registry
    };
}

pub fn get_codec() -> &'static CodecRegistry {
    &CUSTOM_CODEC_REGISTRY
}
