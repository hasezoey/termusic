pub mod buffered_source;
pub mod read_seek_source;

use super::Source;
use std::{fmt, num::NonZeroU64, time::Duration};
use symphonia::{
    core::{
        audio::{Audio, AudioBuffer, AudioSpec, GenericAudioBufferRef},
        codecs::{
            self,
            audio::{well_known::CODEC_ID_MP3, CODEC_ID_NULL_AUDIO},
            CodecParameters,
        },
        errors::Error,
        formats::{
            probe::{Hint, ProbeMetadataData},
            FormatOptions, FormatReader, SeekMode, SeekTo, Track, TrackType,
        },
        io::MediaSourceStream,
        meta::{MetadataOptions, MetadataRevision, StandardTag},
        units::TimeBase,
    },
    default::get_probe,
};
use tokio::sync::mpsc;

fn is_codec_null(track: &Track) -> bool {
    let audio_codec_params = match track.codec_params.as_ref() {
        Some(CodecParameters::Audio(audio)) => audio,
        // treat anything other than "Audio" as null codec
        _ => return true,
    };

    audio_codec_params.codec == CODEC_ID_NULL_AUDIO
}

#[derive(Debug, Clone, PartialEq)]
pub enum MediaTitleType {
    /// Command to instruct storage to clear / reset
    Reset,
    /// Command to provide a new value
    Value(String),
}

pub type MediaTitleRx = mpsc::UnboundedReceiver<MediaTitleType>;
// not public as the transmitter never leaves this module
type MediaTitleTx = mpsc::UnboundedSender<MediaTitleType>;

/// A simple `NewType` to provide extra function for the [`Option<MediaTitleTx>`] type
#[derive(Debug)]
struct MediaTitleTxWrap(Option<MediaTitleTx>);

impl MediaTitleTxWrap {
    pub fn new() -> Self {
        Self(None)
    }

    /// Open and store a new channel
    #[inline]
    pub fn media_title_channel(&mut self) -> MediaTitleRx {
        // unbounded channel so that sending *never* blocks
        let (tx, rx) = mpsc::unbounded_channel();
        self.0 = Some(tx);

        rx
    }

    /// Send a command, without caring if [`None`] or the commands result
    #[inline]
    pub fn media_title_send(&self, cmd: MediaTitleType) {
        if let Some(ref tx) = self.0 {
            let _ = tx.send(cmd);
        }
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// Helper Shorthand to send [`MediaTitleType::Reset`] command
    #[inline]
    pub fn send_reset(&mut self) {
        self.media_title_send(MediaTitleType::Reset);
    }
}

pub struct Symphonia {
    decoder: Box<dyn codecs::audio::AudioDecoder>,
    current_frame_offset: usize,
    probed: Box<dyn FormatReader>,
    buffer: Vec<i16>,
    buffer_frame_len: usize,
    spec: AudioSpec,
    duration: Option<Duration>,
    elapsed: Duration,
    track_id: u32,
    time_base: Option<TimeBase>,
    seek_required_ts: Option<NonZeroU64>,

    media_title_tx: MediaTitleTxWrap,
}

impl Symphonia {
    /// Create a new instance, which also returns a [`MediaTitleRx`]
    #[inline]
    pub fn new_with_media_title(
        mss: MediaSourceStream<'static>,
        gapless: bool,
    ) -> Result<(Self, MediaTitleRx), SymphoniaDecoderError> {
        // guaranteed if "media_title" is set to "true"
        Self::common_new(mss, gapless, true).map(|v| (v.0, v.1.unwrap()))
    }

    /// Create a new instance, without a [`MediaTitleRx`]
    #[inline]
    pub fn new(
        mss: MediaSourceStream<'static>,
        gapless: bool,
    ) -> Result<Self, SymphoniaDecoderError> {
        Self::common_new(mss, gapless, false).map(|v| v.0)
    }

    fn common_new(
        mss: MediaSourceStream<'static>,
        gapless: bool,
        media_title: bool,
    ) -> Result<(Self, Option<MediaTitleRx>), SymphoniaDecoderError> {
        match Self::init(mss, gapless, media_title) {
            Err(e) => match e {
                Error::IoError(e) => Err(SymphoniaDecoderError::IoError(e.to_string())),
                Error::DecodeError(e) => Err(SymphoniaDecoderError::DecodeError(e)),
                Error::SeekError(_) => {
                    unreachable!("Seek errors should not occur during initialization")
                }
                Error::Unsupported(_) => Err(SymphoniaDecoderError::UnrecognizedFormat),
                Error::LimitError(e) => Err(SymphoniaDecoderError::LimitError(e)),
                Error::ResetRequired => Err(SymphoniaDecoderError::ResetRequired),
                _ => todo!("Uncovered Error"),
            },
            Ok(Some((decoder, rx))) => Ok((decoder, rx)),
            Ok(None) => Err(SymphoniaDecoderError::NoStreams),
        }
    }

    fn init(
        mss: MediaSourceStream<'static>,
        gapless: bool,
        media_title: bool,
    ) -> symphonia::core::errors::Result<Option<(Self, Option<MediaTitleRx>)>> {
        let mut probed = get_probe().probe(
            &Hint::default(),
            mss,
            FormatOptions {
                // prebuild_seek_index: true,
                // seek_index_fill_rate: 10,
                enable_gapless: gapless,
                ..Default::default() // enable_gapless: false,
            },
            MetadataOptions::default(),
        )?;

        // see https://github.com/pdeljanov/Symphonia/issues/258
        // TL;DR: "default_track" may choose a video track or a unknown codec, which will fail, this chooses the first non-NULL codec
        // because currently the only way to detect *something* is by comparing the codec_type to NULL
        let track = probed
            .default_track(TrackType::Audio)
            .and_then(|v| if is_codec_null(v) { None } else { Some(v) })
            .or_else(|| probed.tracks().iter().find(|v| !is_codec_null(v)));

        let Some(track) = track else {
            return Ok(None);
        };

        let audio_codec_params = match track.codec_params.as_ref() {
            Some(CodecParameters::Audio(audio)) => audio,
            _ => return Ok(None),
        };

        info!(
            "Found supported container with trackid {} and codectype {}",
            track.id, audio_codec_params.codec
        );

        let mut decoder = symphonia::default::get_codecs().make_audio_decoder(
            &audio_codec_params,
            &codecs::audio::AudioDecoderOptions { verify: true },
        )?;

        let duration = Self::get_duration(&track);
        let track_id = track.id;
        let time_base = track.time_base;
        let mut media_title_tx = MediaTitleTxWrap::new();

        let media_title_rx = if media_title {
            Some(media_title_tx.media_title_channel())
        } else {
            None
        };

        // decode the first part, to get the spec and initial buffer
        let mut buffer = None;
        let Some(DecodeLoopResult { spec, elapsed }) = decode_loop(
            &mut *probed,
            &mut *decoder,
            BufferInputType::New(&mut buffer),
            track_id,
            time_base,
            &mut media_title_tx,
            // &mut probed.metadata(),
            &mut None,
        )?
        else {
            return Ok(None);
        };
        // safe to unwrap because "decode_loop" ensures it will be set
        let buffer = buffer.unwrap();
        let (buffer, buffer_frame_len) = buffer;
        let elapsed = elapsed.unwrap_or_default();

        Ok(Some((
            Self {
                decoder,
                current_frame_offset: 0,
                probed,
                buffer,
                buffer_frame_len,
                spec,
                duration,
                elapsed,
                track_id,
                time_base,
                seek_required_ts: None,

                media_title_tx,
            },
            media_title_rx,
        )))
    }

    fn get_duration(track: &Track) -> Option<Duration> {
        track.num_frames.and_then(|n_frames| {
            track.time_base.map(|tb| {
                let time = tb.calc_time(n_frames);
                time.into()
            })
        })
    }

    /// Copy passed [`GenericAudioBufferRef`] into a new [`AudioBuffer`]
    ///
    /// also see [`Self::maybe_reuse_buffer`]
    #[inline]
    fn get_buffer_new(decoded: GenericAudioBufferRef<'_>) -> (Vec<i16>, usize) {
        let mut buffer = Vec::<i16>::with_capacity(decoded.capacity());
        decoded.copy_to_vec_interleaved(&mut buffer);
        (buffer, decoded.frames())
    }

    /// Copy passed [`GenericAudioBufferRef`] into the existing [`AudioBuffer`], if possible, otherwise create a new
    #[inline]
    fn maybe_reuse_buffer(buffer: (&mut Vec<i16>, &mut usize), decoded: GenericAudioBufferRef<'_>) {
        // calculate what capacity the AudioBuffer will need (as per AudioBuffer internals)
        let required_capacity = decoded.byte_len_as::<i16>();
        // avoid a allocation if not actually necessary
        // this also covers the case if the spec changed from the buffer and decoded
        if required_capacity <= buffer.0.capacity() {
            decoded.copy_to_vec_interleaved(buffer.0);
            *buffer.1 = decoded.frames();
        } else {
            (*buffer.0, *buffer.1) = Self::get_buffer_new(decoded);
        }
    }
}

impl Source for Symphonia {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.buffer_frame_len)
    }

    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn channels(&self) -> u16 {
        self.spec.channels().count() as u16
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.spec.rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.duration
    }

    #[inline]
    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        match self.probed.seek(
            SeekMode::Coarse,
            SeekTo::Time {
                time: pos.into(),
                track_id: Some(self.track_id),
            },
        ) {
            Ok(seeked_to) => {
                // clear sample buffer after seek
                self.current_frame_offset = 0;
                self.buffer.clear();

                // Coarse seeking may seek (slightly) beyond the requested ts, so it may not actually need to be set
                if seeked_to.required_ts > seeked_to.actual_ts {
                    // the unwrap should never fail as "(0 > 0) == false" and "(0 > 1(or higher)) == false"
                    self.seek_required_ts = Some(NonZeroU64::new(seeked_to.required_ts).unwrap());
                }

                // some decoders need to be reset after a seek, but not all can be reset without unexpected behavior (like mka seeking to 0 again)
                // see https://github.com/pdeljanov/Symphonia/issues/274
                if self.decoder.codec_params().codec == CODEC_ID_MP3 {
                    self.decoder.reset();
                }

                Ok(())
            }
            Err(_) => Ok(()),
        }
    }
}

impl Iterator for Symphonia {
    type Item = i16;

    #[inline]
    fn next(&mut self) -> Option<i16> {
        if self.current_frame_offset == self.buffer.len() {
            let DecodeLoopResult { spec, elapsed } = decode_loop(
                &mut *self.probed,
                &mut *self.decoder,
                BufferInputType::Existing((&mut self.buffer, &mut self.buffer_frame_len)),
                self.track_id,
                self.time_base,
                &mut self.media_title_tx,
                // &mut self.probed.metadata,
                &mut self.seek_required_ts,
            )
            .ok()??;

            self.spec = spec;
            if let Some(elapsed) = elapsed {
                self.elapsed = elapsed;
            }

            self.current_frame_offset = 0;
        }

        if self.buffer.is_empty() {
            return None;
        }

        let sample = *self.buffer.get(self.current_frame_offset)?;
        self.current_frame_offset += 1;

        Some(sample)
    }
}

/// Error that can happen when creating a decoder.
#[derive(Debug, Clone)]
pub enum SymphoniaDecoderError {
    /// The format of the data has not been recognized.
    UnrecognizedFormat,

    /// An IO error occured while reading, writing, or seeking the stream.
    IoError(String),

    /// The stream contained malformed data and could not be decoded or demuxed.
    DecodeError(&'static str),

    /// A default or user-defined limit was reached while decoding or demuxing the stream. Limits
    /// are used to prevent denial-of-service attacks from malicious streams.
    LimitError(&'static str),

    /// The demuxer or decoder needs to be reset before continuing.
    ResetRequired,

    /// No streams were found by the decoder
    NoStreams,
}

impl fmt::Display for SymphoniaDecoderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::UnrecognizedFormat => "Unrecognized format",
            Self::IoError(msg) => &msg[..],
            Self::DecodeError(msg) | Self::LimitError(msg) => msg,
            Self::ResetRequired => "Reset required",
            Self::NoStreams => "No streams",
        };
        write!(f, "{text}")
    }
}
impl std::error::Error for SymphoniaDecoderError {}

/// Resulting values from the decode loop
#[derive(Debug)]
struct DecodeLoopResult {
    spec: AudioSpec,
    elapsed: Option<Duration>,
}

// is there maybe a better option for this?
enum BufferInputType<'a> {
    /// Allocate a new [`Vec`] in the specified location (without unsafe)
    New(&'a mut Option<(Vec<i16>, usize)>),
    /// Try to re-use the provided [`Vec`]
    Existing((&'a mut Vec<i16>, &'a mut usize)),
}

impl std::fmt::Debug for BufferInputType<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::New(_) => f.debug_tuple("New").finish(),
            Self::Existing(_) => f.debug_tuple("Existing").finish(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
/// Decode until finding a valid packet and get the samples from it
///
/// If [`BufferInputType::New`] is used, it is guaranteed to be [`Some`] if function result is [`Ok`].
fn decode_loop(
    format: &mut dyn FormatReader,
    decoder: &mut dyn codecs::audio::AudioDecoder,
    buffer: BufferInputType<'_>,
    track_id: u32,
    time_base: Option<TimeBase>,
    media_title_tx: &mut MediaTitleTxWrap,
    // probed: &mut ProbeMetadataData,
    seek_required_ts: &mut Option<NonZeroU64>,
) -> Result<Option<DecodeLoopResult>, symphonia::core::errors::Error> {
    let (audio_buf, elapsed) = loop {
        let Some(packet) = format.next_packet()? else {
            return Ok(None);
        };

        // Skip all packets that are not the selected track
        if packet.track_id() != track_id {
            continue;
        }

        // seeking in symphonia can only be done to the nearest packet in the format reader
        // so we need to also seek until the actually required_ts in the decoder
        if let Some(dur) = seek_required_ts {
            if packet.ts() < dur.get() {
                continue;
            }
            // else, remove the value as we are now at or beyond that point
            seek_required_ts.take();
        }

        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                let ts = packet.ts();
                let elapsed = time_base.map(|tb| Duration::from(tb.calc_time(ts)));
                break (audio_buf, elapsed);
            }
            Err(Error::DecodeError(err)) => {
                info!("Non-fatal Decoder Error: {}", err);
            }
            Err(err) => return Err(err),
        }
    };

    if !media_title_tx.is_none() {
        // run container metadata if new and on seek to 0
        // this maybe not be 100% reliable, where for example there is no "time_base", but i dont know of a case yet where that happens
        if elapsed.as_ref().map_or(false, Duration::is_zero) {
            trace!("Time is 0, doing container metadata");
            media_title_tx.send_reset();
            do_container_metdata(media_title_tx, format /* probed */);
        } else if !format.metadata().is_latest() {
            // only execute it once if there is a new metadata iteration
            do_inline_metdata(media_title_tx, format);
        }
    }

    let spec = audio_buf.spec().clone();

    match buffer {
        BufferInputType::New(buffer) => {
            *buffer = Some(Symphonia::get_buffer_new(audio_buf));
        }
        BufferInputType::Existing(buffer) => {
            Symphonia::maybe_reuse_buffer(buffer, audio_buf);
        }
    }

    Ok(Some(DecodeLoopResult { spec, elapsed }))
}

/// Do container metadata / track start metadata
///
/// No optimizations for when [`MediaTitleTxWrap`] is [`None`], should be done outside of this function
#[inline]
fn do_container_metdata(
    media_title_tx: &mut MediaTitleTxWrap,
    format: &mut dyn FormatReader,
    // probed: &mut ProbeMetadataData,
) {
    // prefer standard container tags over non-standard
    let title = if let Some(metadata_rev) = format.metadata().current() {
        // tags that are from the container standard (like mkv)
        find_title_metadata(metadata_rev).cloned()
    }
    /* else if let Some(metadata_rev) = probed.get().as_ref().and_then(|m| m.current()) {
        // tags that are not from the container standard (like mp3)
        find_title_metadata(metadata_rev).cloned()
    } */
    else {
        trace!("Did not find any metadata in either format or probe!");
        None
    };

    // TODO: maybe change things if https://github.com/pdeljanov/Symphonia/issues/273 should not get unified into metadata

    if let Some(title) = title {
        media_title_tx.media_title_send(MediaTitleType::Value(title));
    }
}

/// Some containers support updating / setting metadata as a frame somewhere inside the track, which could be used for live-streams
///
/// No optimizations for when [`MediaTitleTxWrap`] is [`None`], should be done outside of this function
#[inline]
fn do_inline_metdata(media_title_tx: &mut MediaTitleTxWrap, format: &mut dyn FormatReader) {
    if let Some(metadata_rev) = format.metadata().skip_to_latest() {
        if let Some(title) = find_title_metadata(metadata_rev).cloned() {
            media_title_tx.media_title_send(MediaTitleType::Value(title));
        }
    }
}

#[inline]
fn find_title_metadata<'a>(metadata: &'a MetadataRevision) -> Option<&'a String> {
    let t = metadata.tags().iter().find_map(|v| {
        v.std.as_ref().and_then(|v| match v {
            StandardTag::TrackTitle(title) => Some(&**title),
            _ => None,
        })
    });

    t
}
