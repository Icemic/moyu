// This file is based on code from the Kira audio library
// Original source: https://github.com/tesselode/kira
// Original copyright: Copyright (c) 2020 tesselode
// Original license: MIT OR Apache-2.0
//
// Modifications made to support custom Symphonia codec registration.
// Modified code is licensed under the Mozilla Public License Version 2.0.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use kira::Frame;
use kira::sound::static_sound::StaticSoundData;
use kira::sound::{FromFileError, static_sound::StaticSoundSettings};
use symphonia::core::formats::TrackType;
use symphonia::core::{
    audio::GenericAudioBufferRef,
    codecs::CodecParameters,
    formats::probe::Hint,
    io::{MediaSource, MediaSourceStream},
};

use moyu_pal::symphonia::get_codec;

pub fn from_boxed_media_source(
    media_source: Box<dyn MediaSource>,
) -> Result<StaticSoundData, FromFileError> {
    // let codecs = symphonia::default::get_codecs();
    let codecs = get_codec();
    let probe = symphonia::default::get_probe();
    let mss = MediaSourceStream::new(media_source, Default::default());
    let mut format_reader =
        probe.probe(&Hint::new(), mss, Default::default(), Default::default())?;
    let default_track = format_reader
        .first_track(TrackType::Audio)
        .ok_or(FromFileError::NoDefaultTrack)?;
    let default_track_id = default_track.id;
    let codec_params = match default_track.codec_params.as_ref() {
        Some(CodecParameters::Audio(params)) => params.clone(),
        _ => return Err(FromFileError::NoDefaultTrack),
    };
    let sample_rate = codec_params
        .sample_rate
        .ok_or(FromFileError::UnknownSampleRate)?;
    let mut decoder = codecs.make_audio_decoder(&codec_params, &Default::default())?;
    let mut frames = vec![];
    loop {
        match format_reader.next_packet() {
            Ok(Some(packet)) => {
                if default_track_id == packet.track_id {
                    let buffer = decoder.decode(&packet)?;
                    frames.append(&mut load_frames_from_buffer_ref(&buffer)?);
                }
            }
            Ok(None) => break,
            Err(error) => match error {
                symphonia::core::errors::Error::IoError(error) => {
                    if error.kind() == std::io::ErrorKind::UnexpectedEof {
                        break;
                    }
                    return Err(symphonia::core::errors::Error::IoError(error).into());
                }
                error => return Err(error.into()),
            },
        }
    }
    Ok(StaticSoundData {
        sample_rate,
        frames: frames.into(),
        settings: StaticSoundSettings::default(),
        slice: None,
    })
}

pub fn load_frames_from_buffer_ref(
    buffer: &GenericAudioBufferRef<'_>,
) -> Result<Vec<Frame>, FromFileError> {
    let mut samples = Vec::with_capacity(buffer.samples_interleaved());
    buffer.copy_to_vec_interleaved(&mut samples);

    match buffer.spec().channels().count() {
        1 => Ok(samples.into_iter().map(Frame::from_mono).collect()),
        2 => Ok(samples
            .chunks_exact(2)
            .map(|samples| Frame::new(samples[0], samples[1]))
            .collect()),
        _ => Err(FromFileError::UnsupportedChannelConfiguration),
    }
}
