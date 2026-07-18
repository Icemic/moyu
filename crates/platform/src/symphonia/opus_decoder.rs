use symphonia::core::audio::{
    AsGenericAudioBufferRef, AudioBuffer, AudioMut, AudioSpec, GenericAudioBufferRef,
};
use symphonia::core::codecs::CodecInfo;
use symphonia::core::codecs::audio::{
    AudioCodecParameters, AudioDecoder, AudioDecoderOptions, FinalizeResult,
    well_known::CODEC_ID_OPUS,
};
use symphonia::core::codecs::registry::{RegisterableAudioDecoder, SupportedAudioCodec};
use symphonia::core::errors::{Result, unsupported_error};
use symphonia::core::packet::PacketRef;

const DEFAULT_FRAME_SIZE: usize = 960; // 20ms at 48kHz

pub struct OpusDecoder {
    options: AudioDecoderOptions,
    params: AudioCodecParameters,
    spec: AudioSpec,
    channels: u16,
    frame_size: usize,
    decoder: opus::Decoder,
    buffer: AudioBuffer<f32>,
    cache: [f32; DEFAULT_FRAME_SIZE * 2],
}

unsafe impl Send for OpusDecoder {}
unsafe impl Sync for OpusDecoder {}

impl OpusDecoder {
    fn try_new(params: &AudioCodecParameters, options: &AudioDecoderOptions) -> Result<Self> {
        if params.codec != CODEC_ID_OPUS {
            return unsupported_error("opus: invalid codec");
        }

        let sample_rate = params
            .sample_rate
            .ok_or(symphonia::core::errors::Error::Unsupported(
                "Sample rate is required",
            ))?;

        let channel_layout =
            params
                .channels
                .clone()
                .ok_or(symphonia::core::errors::Error::Unsupported(
                    "Channel count is required",
                ))?;
        let channels = channel_layout.count() as u16;

        let frame_size = DEFAULT_FRAME_SIZE;

        let decoder = opus::Decoder::new(
            sample_rate as u32,
            match channels {
                1 => opus::Channels::Mono,
                2 => opus::Channels::Stereo,
                _ => {
                    return Err(symphonia::core::errors::Error::Unsupported(
                        "Only mono and stereo are supported",
                    ));
                }
            },
        )
        .map_err(|e| {
            log::error!("Failed to create Opus decoder: {}", e);
            symphonia::core::errors::Error::DecodeError("Failed to create Opus decoder")
        })?;

        let spec = AudioSpec::new(sample_rate, channel_layout);
        let buffer = AudioBuffer::<f32>::new(spec.clone(), frame_size);

        Ok(OpusDecoder {
            options: *options,
            params: params.clone(),
            spec,
            channels,
            frame_size,
            decoder,
            buffer,
            cache: [0.; DEFAULT_FRAME_SIZE * 2],
        })
    }
}

impl AudioDecoder for OpusDecoder {
    fn reset(&mut self) {
        if let Err(err) = self.decoder.reset_state() {
            log::error!("Failed to reset Opus decoder state: {}", err);
        }
    }

    fn codec_info(&self) -> &CodecInfo {
        &Self::supported_codecs()[0].info
    }

    fn codec_params(&self) -> &AudioCodecParameters {
        &self.params
    }

    fn decode_ref(&mut self, packet: &PacketRef<'_>) -> Result<GenericAudioBufferRef<'_>> {
        let decoded_frame_size =
            match self
                .decoder
                .decode_float(&packet.data, &mut self.cache, false)
            {
                Ok(frame_size) => frame_size,
                Err(error) => {
                    self.buffer.clear();
                    log::error!("Failed to decode Opus packet: {}", error);
                    return Err(symphonia::core::errors::Error::DecodeError("Decode error"));
                }
            };

        if decoded_frame_size != self.frame_size {
            log::info!(
                "Opus frame size changed from {} to {}",
                self.frame_size,
                decoded_frame_size
            );
            self.buffer = AudioBuffer::<f32>::new(self.spec.clone(), decoded_frame_size);
            self.frame_size = decoded_frame_size;
        }

        let actual_samples = decoded_frame_size * self.channels as usize;
        let data = &self.cache[..actual_samples];

        self.buffer.clear();
        self.buffer.render_uninit(Some(decoded_frame_size));

        for (i, sample) in data.iter().enumerate() {
            let channel = i % self.channels as usize;
            self.buffer.plane_mut(channel).unwrap()[i / self.channels as usize] = *sample;
        }

        if self.options.gapless {
            self.buffer.trim(
                packet.trim_start.get() as usize,
                packet.trim_end.get() as usize,
            );
        }

        Ok(self.buffer.as_generic_audio_buffer_ref())
    }

    fn finalize(&mut self) -> FinalizeResult {
        Default::default()
    }

    fn last_decoded(&self) -> GenericAudioBufferRef<'_> {
        self.buffer.as_generic_audio_buffer_ref()
    }
}

impl RegisterableAudioDecoder for OpusDecoder {
    fn try_registry_new(
        params: &AudioCodecParameters,
        options: &AudioDecoderOptions,
    ) -> Result<Box<dyn AudioDecoder>> {
        Ok(Box::new(OpusDecoder::try_new(params, options)?))
    }

    fn supported_codecs() -> &'static [SupportedAudioCodec] {
        &[SupportedAudioCodec {
            id: CODEC_ID_OPUS,
            info: CodecInfo {
                short_name: "opus",
                long_name: "Opus",
                profiles: &[],
            },
        }]
    }
}
