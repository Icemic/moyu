use symphonia::core::audio::AsAudioBufferRef;
use symphonia::core::audio::AudioBuffer;
use symphonia::core::audio::Layout;
use symphonia::core::audio::Signal;
use symphonia::core::audio::SignalSpec;
use symphonia::core::codecs::*;
use symphonia::core::support_codec;

const DEFAULT_FRAME_SIZE: usize = 960; // 20ms at 48kHz

pub struct OpusDecoder {
    params: CodecParameters,
    sample_rate: u32,
    channels: u16,
    frame_size: usize,
    decoder: opus::Decoder,
    buffer: AudioBuffer<f32>,
    cache: [f32; DEFAULT_FRAME_SIZE * 2],
}

unsafe impl Send for OpusDecoder {}
unsafe impl Sync for OpusDecoder {}

impl symphonia::core::codecs::Decoder for OpusDecoder {
    fn try_new(
        params: &CodecParameters,
        _options: &DecoderOptions,
    ) -> symphonia::core::errors::Result<Self>
    where
        Self: Sized,
    {
        let sample_rate = params
            .sample_rate
            .ok_or(symphonia::core::errors::Error::Unsupported(
                "Sample rate is required",
            ))?;

        let channels = params.channels.map(|c| c.count() as u16).ok_or(
            symphonia::core::errors::Error::Unsupported("Channel count is required"),
        )?;

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

        let buffer = AudioBuffer::<f32>::new(
            frame_size as u64,
            SignalSpec::new(
                sample_rate,
                match channels {
                    1 => Layout::Mono.into_channels(),
                    2 => Layout::Stereo.into_channels(),
                    _ => {
                        return Err(symphonia::core::errors::Error::Unsupported(
                            "Only mono and stereo are supported",
                        ));
                    }
                },
            ),
        );

        Ok(OpusDecoder {
            params: params.clone(),
            sample_rate,
            channels,
            frame_size,
            decoder,
            buffer,
            cache: [0.; DEFAULT_FRAME_SIZE * 2],
        })
    }

    fn supported_codecs() -> &'static [CodecDescriptor]
    where
        Self: Sized,
    {
        &[support_codec!(CODEC_TYPE_OPUS, "opus", "opus")]
    }

    fn reset(&mut self) {
        if let Err(err) = self.decoder.reset_state() {
            log::error!("Failed to reset Opus decoder state: {}", err);
        }
    }

    fn codec_params(&self) -> &CodecParameters {
        &self.params
    }

    fn last_decoded(&self) -> symphonia::core::audio::AudioBufferRef<'_> {
        self.buffer.as_audio_buffer_ref()
    }

    fn decode(
        &mut self,
        packet: &symphonia::core::formats::Packet,
    ) -> Result<symphonia::core::audio::AudioBufferRef<'_>, symphonia::core::errors::Error> {
        let decoded_frame_size = self
            .decoder
            .decode_float(&packet.data, &mut self.cache, false)
            .map_err(|e| {
                log::error!("Failed to decode Opus packet: {}", e);
                symphonia::core::errors::Error::DecodeError("Decode error")
            })?;

        if decoded_frame_size != self.frame_size {
            log::info!(
                "Opus frame size changed from {} to {}",
                self.frame_size,
                decoded_frame_size
            );
            self.buffer = AudioBuffer::<f32>::new(
                decoded_frame_size as u64,
                SignalSpec::new(
                    self.sample_rate,
                    match self.channels {
                        1 => Layout::Mono.into_channels(),
                        2 => Layout::Stereo.into_channels(),
                        _ => {
                            return Err(symphonia::core::errors::Error::Unsupported(
                                "Only mono and stereo are supported",
                            ));
                        }
                    },
                ),
            );
            self.frame_size = decoded_frame_size;
        }

        let actual_samples = decoded_frame_size * self.channels as usize;
        let data = &self.cache[..actual_samples];

        self.buffer.clear();
        self.buffer.render_reserved(None);

        for (i, sample) in data.iter().enumerate() {
            let channel = i % self.channels as usize;
            self.buffer.chan_mut(channel)[i / self.channels as usize] = *sample;
        }

        self.buffer
            .trim(packet.trim_start as usize, packet.trim_end as usize);

        Ok(self.buffer.as_audio_buffer_ref())
    }

    fn finalize(&mut self) -> FinalizeResult {
        FinalizeResult { verify_ok: None }
    }
}
