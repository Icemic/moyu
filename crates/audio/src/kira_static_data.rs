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
use symphonia::core::{
    audio::{AudioBuffer, AudioBufferRef, Signal},
    conv::{FromSample, IntoSample},
    io::{MediaSource, MediaSourceStream},
    sample::Sample,
};

use moyu_pal::symphonia::get_codec;

pub fn from_boxed_media_source(
    media_source: Box<dyn MediaSource>,
) -> Result<StaticSoundData, FromFileError> {
    // let codecs = symphonia::default::get_codecs();
    let codecs = get_codec();
    let probe = symphonia::default::get_probe();
    let mss = MediaSourceStream::new(media_source, Default::default());
    let mut format_reader = probe
        .format(
            &Default::default(),
            mss,
            &Default::default(),
            &Default::default(),
        )?
        .format;
    let default_track = format_reader
        .default_track()
        .ok_or(FromFileError::NoDefaultTrack)?;
    let default_track_id = default_track.id;
    let codec_params = &default_track.codec_params;
    let sample_rate = codec_params
        .sample_rate
        .ok_or(FromFileError::UnknownSampleRate)?;
    let mut decoder = codecs.make(codec_params, &Default::default())?;
    let mut frames = vec![];
    loop {
        match format_reader.next_packet() {
            Ok(packet) => {
                if default_track_id == packet.track_id() {
                    let buffer = decoder.decode(&packet)?;
                    frames.append(&mut load_frames_from_buffer_ref(&buffer)?);
                }
            }
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

pub fn load_frames_from_buffer_ref(buffer: &AudioBufferRef) -> Result<Vec<Frame>, FromFileError> {
    match buffer {
        AudioBufferRef::U8(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::U16(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::U24(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::U32(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::S8(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::S16(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::S24(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::S32(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::F32(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::F64(buffer) => load_frames_from_buffer(buffer),
    }
}

pub fn load_frames_from_buffer<S: Sample>(
    buffer: &AudioBuffer<S>,
) -> Result<Vec<Frame>, FromFileError>
where
    f32: FromSample<S>,
{
    match buffer.spec().channels.count() {
        1 => Ok(buffer
            .chan(0)
            .iter()
            .map(|sample| Frame::from_mono((*sample).into_sample()))
            .collect()),
        2 => Ok(buffer
            .chan(0)
            .iter()
            .zip(buffer.chan(1).iter())
            .map(|(left, right)| Frame::new((*left).into_sample(), (*right).into_sample()))
            .collect()),
        _ => Err(FromFileError::UnsupportedChannelConfiguration),
    }
}
