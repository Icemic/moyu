use std::collections::VecDeque;
use std::io::Cursor;

use anyhow::{Result, anyhow};
use symphonia::core::codecs::{CODEC_TYPE_NULL, CodecParameters};
use symphonia::core::formats::{FormatReader, Packet, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

use crate::types::VideoCodec;

/// Try to identify a video codec from codec private data (extra_data).
/// AV1CodecConfigurationRecord always starts with 0x81 (marker bit + version=1).
/// VPCodecConfigurationRecord starts with 0x00-0x06 (profile 0-3). No overlap.
fn detect_codec_from_extra_data(data: &[u8]) -> Option<VideoCodec> {
    match data.first() {
        Some(&0x81) => Some(VideoCodec::Av1),
        _ => None,
    }
}

/// Try to identify a video codec by looking at the first packet data.
fn detect_codec_from_packet(data: &[u8]) -> Option<VideoCodec> {
    let b = *data.first()?;

    // AV1 OBU header:
    // forbidden bit (7) must be 0, reserved bit (0) must be 0
    if (b & 0x81) == 0 {
        let obu_type = (b >> 3) & 0x0F;
        // Types: 1=Sequence Header, 2=Temporal Delimiter, etc.
        if obu_type > 0 && obu_type <= 8 {
            return Some(VideoCodec::Av1);
        }
    }

    // VP9 uncompressed header:
    // Frame marker is 0b10 in the MSBs (bits 7 and 6)
    if (b & 0xC0) == 0x80 {
        return Some(VideoCodec::Vp9);
    }

    None
}

/// Track type identified during probing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    Video,
    Audio,
}

/// Information about a track in the container
#[derive(Debug)]
pub struct TrackInfo {
    id: u32,
    #[allow(dead_code)]
    kind: TrackKind,
    codec_params: CodecParameters,
    /// For video tracks, the identified codec
    #[allow(dead_code)]
    video_codec: Option<VideoCodec>,
}

/// Demuxer using Symphonia for container format parsing (MP4, MKV, WebM).
pub struct Demuxer {
    reader: Box<dyn FormatReader>,
    video_track_id: Option<u32>,
    audio_track_id: Option<u32>,
    video_codec: Option<VideoCodec>,
    tracks: Vec<TrackInfo>,
    duration_secs: Option<f64>,
    /// Packets buffered during container probing
    buffered_packets: VecDeque<(TrackKind, Packet)>,
}

impl Demuxer {
    /// Open a media file from raw data bytes.
    /// The `hint` should be the file extension (e.g. "webm", "mkv", "mp4").
    /// The `video_codec` hint tells which video codec to expect (VP9 or AV1),
    /// since symphonia is audio-focused and doesn't identify video codecs.
    pub fn open(data: Vec<u8>, hint: Option<&str>) -> Result<Self> {
        let cursor = Cursor::new(data);
        let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

        let mut probe_hint = Hint::new();
        if let Some(ext) = hint {
            probe_hint.with_extension(ext);
        }

        let probe = moyu_pal::symphonia::get_probe();

        let probed = probe
            .format(
                &probe_hint,
                mss,
                &Default::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| anyhow!("Failed to probe media format: {}", e))?;

        let mut reader = probed.format;

        let mut video_track_id = None;
        let mut audio_track_id = None;
        let mut tracks = Vec::new();
        let mut duration_secs = None;
        let mut video_codec = None;

        // Symphonia is audio-focused. Video tracks appear as tracks with
        // CODEC_TYPE_NULL (unrecognized codec) and no sample_rate.
        // Audio tracks have a recognized codec type and/or sample_rate.
        for track in reader.tracks() {
            let codec_type = track.codec_params.codec;
            let has_sample_rate = track.codec_params.sample_rate.is_some();
            let is_null_codec = codec_type == CODEC_TYPE_NULL;

            // Video track: null codec and no sample rate (not audio)
            if is_null_codec && !has_sample_rate {
                if video_track_id.is_none() {
                    video_track_id = Some(track.id);
                    if video_codec.is_none() {
                        video_codec = track
                            .codec_params
                            .extra_data
                            .as_deref()
                            .and_then(detect_codec_from_extra_data);
                    }
                }

                tracks.push(TrackInfo {
                    id: track.id,
                    kind: TrackKind::Video,
                    codec_params: track.codec_params.clone(),
                    video_codec,
                });
            } else {
                // Audio track
                if audio_track_id.is_none() {
                    audio_track_id = Some(track.id);
                }

                tracks.push(TrackInfo {
                    id: track.id,
                    kind: TrackKind::Audio,
                    codec_params: track.codec_params.clone(),
                    video_codec: None,
                });
            }

            // Try to get duration from track params
            if duration_secs.is_none() {
                if let Some(n_frames) = track.codec_params.n_frames {
                    if let Some(tb) = track.codec_params.time_base {
                        let d = tb.calc_time(n_frames);
                        duration_secs = Some(d.seconds as f64 + d.frac);
                    }
                }
            }
        }

        let mut buffered_packets = VecDeque::new();

        // If we still don't know the video codec, we probe the first video packet
        if video_codec.is_none() && video_track_id.is_some() {
            for _ in 0..50 {
                match reader.next_packet() {
                    Ok(packet) => {
                        let track_id = packet.track_id();
                        let mut pkt_kind = None;
                        if Some(track_id) == video_track_id {
                            pkt_kind = Some(TrackKind::Video);
                        } else if Some(track_id) == audio_track_id {
                            pkt_kind = Some(TrackKind::Audio);
                        }

                        if let Some(kind) = pkt_kind {
                            let is_video = kind == TrackKind::Video;
                            buffered_packets.push_back((kind, packet));

                            if is_video {
                                let pkt = &buffered_packets.back().unwrap().1;
                                video_codec = detect_codec_from_packet(&pkt.data);
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        // Final fallback to VP9 if all detection methods failed
        let final_video_codec = video_codec.or(Some(VideoCodec::Vp9));

        // Update the video_codec in TrackInfo
        for track in &mut tracks {
            if track.kind == TrackKind::Video {
                track.video_codec = final_video_codec;
            }
        }

        Ok(Self {
            reader,
            video_track_id,
            audio_track_id,
            video_codec: final_video_codec,
            tracks,
            duration_secs,
            buffered_packets,
        })
    }

    /// Get the identified video codec, if any.
    pub fn video_codec(&self) -> Option<VideoCodec> {
        self.video_codec
    }

    /// Get codec parameters for the audio track.
    pub fn audio_codec_params(&self) -> Option<&CodecParameters> {
        self.audio_track_id.and_then(|id| {
            self.tracks
                .iter()
                .find(|t| t.id == id)
                .map(|t| &t.codec_params)
        })
    }

    /// Get the total duration of the media in seconds, if known.
    pub fn duration(&self) -> Option<f64> {
        self.duration_secs
    }

    /// Read the next packet from the container.
    /// Returns (track_kind, packet) where track_kind indicates whether
    /// the packet belongs to video or audio.
    pub fn next_packet(&mut self) -> Result<Option<(TrackKind, Packet)>> {
        if let Some((kind, packet)) = self.buffered_packets.pop_front() {
            return Ok(Some((kind, packet)));
        }

        loop {
            match self.reader.next_packet() {
                Ok(packet) => {
                    let track_id = packet.track_id();

                    if Some(track_id) == self.video_track_id {
                        return Ok(Some((TrackKind::Video, packet)));
                    } else if Some(track_id) == self.audio_track_id {
                        return Ok(Some((TrackKind::Audio, packet)));
                    }
                    // Skip packets for tracks we don't care about
                    continue;
                }
                Err(symphonia::core::errors::Error::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None);
                }
                Err(e) => return Err(anyhow!("Error reading packet: {}", e)),
            }
        }
    }

    /// Seek to a position in seconds.
    pub fn seek(&mut self, time_secs: f64) -> Result<()> {
        let seek_to = SeekTo::Time {
            time: Time::new(time_secs as u64, time_secs.fract()),
            track_id: None, // seek all tracks
        };

        self.reader
            .seek(SeekMode::Coarse, seek_to)
            .map_err(|e| anyhow!("Seek failed: {}", e))?;

        Ok(())
    }

    /// Get the time base for the video track, converting packet timestamps to seconds.
    pub fn video_time_base(&self) -> Option<(u32, u32)> {
        self.video_track_id.and_then(|id| {
            self.tracks
                .iter()
                .find(|t| t.id == id)
                .and_then(|t| t.codec_params.time_base.map(|tb| (tb.numer, tb.denom)))
        })
    }

    /// Get the time base for the audio track.
    pub fn audio_time_base(&self) -> Option<(u32, u32)> {
        self.audio_track_id.and_then(|id| {
            self.tracks
                .iter()
                .find(|t| t.id == id)
                .and_then(|t| t.codec_params.time_base.map(|tb| (tb.numer, tb.denom)))
        })
    }
}
