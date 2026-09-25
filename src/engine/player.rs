//! Soundboard and music player that feeds the PLAYER strip. Pads are fire-and-forget voices that
//! may overlap; the music track is a single voice with pause, seek and loop.

use crate::audio::resample::LinearResampler;
use crate::SAMPLE_RATE;
use anyhow::{anyhow, Context, Result};
use crossbeam_channel::{Receiver, Sender};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use crate::dsp::meter::{block_peak, AtomicPeak, MeterBallistics};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL, CODEC_TYPE_OPUS};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Decoded audio, interleaved stereo at the engine rate.
#[derive(Clone, Debug)]
pub struct Clip {
    pub name: String,
    pub samples: Arc<Vec<f32>>,
}

impl Clip {
    pub fn frames(&self) -> usize {
        self.samples.len() / 2
    }

    pub fn duration_secs(&self) -> f32 {
        self.frames() as f32 / SAMPLE_RATE as f32
    }

    /// Builds a clip from already-decoded PCM, converting channel count and rate.
    pub fn from_pcm(name: &str, pcm: &[f32], channels: usize, rate: u32) -> Self {
        let mut stereo = Vec::with_capacity(pcm.len() / channels.max(1) * 2);
        crate::audio::streams::to_stereo(pcm, channels, &mut stereo);
        let samples = if rate == SAMPLE_RATE {
            stereo
        } else {
            let mut r = LinearResampler::new(rate, SAMPLE_RATE);
            r.push(&stereo);
            let mut out = vec![0.0; r.available_out() * 2];
            r.pull(&mut out);
            out
        };
        Self { name: name.to_string(), samples: Arc::new(samples) }
    }
}

/// Decodes MP3, WAV, FLAC, OGG, AAC/M4A with symphonia.
/// Shown when an .ogg (or .opus) file holds Opus rather than Vorbis audio.
pub const OPUS_UNSUPPORTED: &str = "Opus audio is not supported yet (Discord and WhatsApp clips use it). Convert it to MP3, WAV, FLAC or OGG Vorbis.";

pub fn load_clip(path: &Path) -> Result<Clip> {
    let file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .context("unsupported or corrupt audio file")?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("no audio track"))?;
    let track_id = track.id;
    // The Ogg demuxer knows Opus streams (Discord voice clips, WhatsApp notes) but no Opus
    // decoder ships with the app, so name the problem instead of "unsupported codec".
    if track.codec_params.codec == CODEC_TYPE_OPUS {
        return Err(anyhow!(OPUS_UNSUPPORTED));
    }
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("no decoder for this codec")?;

    let mut pcm: Vec<f32> = Vec::new();
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut rate = track.codec_params.sample_rate.unwrap_or(SAMPLE_RATE);
    let mut channels = 2;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymphoniaError::ResetRequired) => break,
            Err(e) => return Err(e.into()),
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let buf = sample_buf.get_or_insert_with(|| {
                    let spec = *decoded.spec();
                    rate = spec.rate;
                    channels = spec.channels.count();
                    SampleBuffer::<f32>::new(decoded.capacity() as u64, spec)
                });
                buf.copy_interleaved_ref(decoded);
                pcm.extend_from_slice(buf.samples());
            }
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(e.into()),
        }
    }
    if pcm.is_empty() {
        return Err(anyhow!("file decoded to no audio"));
    }
    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("clip");
    Ok(Clip::from_pcm(name, &pcm, channels, rate))
}

pub enum PlayerCommand {
    /// Plays a pad; a pad that is already sounding is restarted.
    PlayPad { pad: usize, samples: Arc<Vec<f32>> },
    StopPads,
    /// Linear gain applied to every sounding pad, live.
    PadsGain(f32),
    LoadMusic { samples: Arc<Vec<f32>> },
    MusicPlayPause,
    MusicStop,
    /// 0..1 of the track.
    MusicSeek(f32),
    MusicLoop(bool),
    MusicGain(f32),
}

/// Playback state published for the UI.
#[derive(Debug, Default)]
pub struct PlayerStatus {
    pub music_position: AtomicU32,
    pub music_length: AtomicU32,
    pub music_playing: AtomicBool,
    pub pads_sounding: AtomicU32,
    /// Smoothed peaks of the music and of the pads on their own, before the PLAYER strip.
    pub music_peak: AtomicPeak,
    pub pads_peak: AtomicPeak,
}

struct PadVoice {
    pad: usize,
    samples: Arc<Vec<f32>>,
    pos: usize,
}

struct MusicVoice {
    samples: Arc<Vec<f32>>,
    pos: usize,
    playing: bool,
}

pub struct Player {
    rx: Receiver<PlayerCommand>,
    status: Arc<PlayerStatus>,
    pads: Vec<PadVoice>,
    music: Option<MusicVoice>,
    music_loop: bool,
    music_gain: f32,
    pads_gain: f32,
    music_meter: MeterBallistics,
    pads_meter: MeterBallistics,
    /// The music block on its own, so it can be metered before it joins the pads.
    scratch: Vec<f32>,
}

impl Player {
    pub fn new() -> (Self, Sender<PlayerCommand>, Arc<PlayerStatus>) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let status = Arc::new(PlayerStatus::default());
        let player = Self {
            rx,
            status: status.clone(),
            pads: Vec::new(),
            music: None,
            music_loop: false,
            music_gain: 1.0,
            pads_gain: 1.0,
            music_meter: MeterBallistics::default(),
            pads_meter: MeterBallistics::default(),
            scratch: Vec::new(),
        };
        (player, tx, status)
    }

    fn handle(&mut self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::PlayPad { pad, samples } => {
                self.pads.retain(|v| v.pad != pad);
                self.pads.push(PadVoice { pad, samples, pos: 0 });
            }
            PlayerCommand::StopPads => self.pads.clear(),
            PlayerCommand::PadsGain(g) => self.pads_gain = g.max(0.0),
            PlayerCommand::LoadMusic { samples } => {
                // An empty clip has nothing to play and would spin the loop below when looping.
                self.music = (!samples.is_empty()).then_some(MusicVoice { samples, pos: 0, playing: true });
            }
            PlayerCommand::MusicPlayPause => {
                if let Some(m) = &mut self.music {
                    m.playing = !m.playing;
                }
            }
            PlayerCommand::MusicStop => {
                if let Some(m) = &mut self.music {
                    m.playing = false;
                    m.pos = 0;
                }
            }
            PlayerCommand::MusicSeek(t) => {
                if let Some(m) = &mut self.music {
                    let frames = m.samples.len() / 2;
                    m.pos = ((t.clamp(0.0, 1.0) * frames as f32) as usize).min(frames.saturating_sub(1));
                }
            }
            PlayerCommand::MusicLoop(on) => self.music_loop = on,
            PlayerCommand::MusicGain(g) => self.music_gain = g.max(0.0),
        }
    }

    /// Writes one block of mixed pads + music into `out` (interleaved stereo, overwritten), and
    /// meters the pads and the music separately.
    pub fn render(&mut self, out: &mut [f32]) {
        while let Ok(cmd) = self.rx.try_recv() {
            self.handle(cmd);
        }
        out.fill(0.0);
        for voice in &mut self.pads {
            let end = (voice.pos + out.len()).min(voice.samples.len());
            for (o, s) in out.iter_mut().zip(voice.samples[voice.pos..end].iter()) {
                *o += s * self.pads_gain;
            }
            voice.pos = end;
        }
        self.pads.retain(|v| v.pos < v.samples.len());
        let pads_peak = self.pads_meter.update(block_peak(out));
        self.status.pads_peak.store(pads_peak[0], pads_peak[1]);

        self.scratch.clear();
        self.scratch.resize(out.len(), 0.0);
        if let Some(m) = &mut self.music {
            if m.playing {
                let mut written = 0;
                while written < out.len() && !m.samples.is_empty() {
                    let end = (m.pos + out.len() - written).min(m.samples.len());
                    for (o, s) in self.scratch[written..].iter_mut().zip(m.samples[m.pos..end].iter()) {
                        *o += s * self.music_gain;
                    }
                    written += end - m.pos;
                    m.pos = end;
                    if m.pos >= m.samples.len() {
                        if self.music_loop {
                            m.pos = 0;
                        } else {
                            m.playing = false;
                            m.pos = 0;
                            break;
                        }
                    }
                }
            }
            self.status.music_position.store((m.pos / 2) as u32, Ordering::Relaxed);
            self.status.music_length.store((m.samples.len() / 2) as u32, Ordering::Relaxed);
            self.status.music_playing.store(m.playing, Ordering::Relaxed);
        }
        let music_peak = self.music_meter.update(block_peak(&self.scratch));
        self.status.music_peak.store(music_peak[0], music_peak[1]);
        for (o, s) in out.iter_mut().zip(self.scratch.iter()) {
            *o += s;
        }
        self.status.pads_sounding.store(self.pads.len() as u32, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(value: f32, frames: usize) -> Arc<Vec<f32>> {
        Arc::new(vec![value; frames * 2])
    }

    #[test]
    fn pad_plays_once_and_stops() {
        let (mut p, tx, status) = Player::new();
        tx.send(PlayerCommand::PlayPad { pad: 0, samples: clip(0.5, 6) }).unwrap();
        let mut out = vec![0.0; 8];
        p.render(&mut out);
        assert_eq!(out, vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]);
        p.render(&mut out);
        assert_eq!(out, vec![0.5, 0.5, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(status.pads_sounding.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn pads_overlap_and_music_loops() {
        let (mut p, tx, status) = Player::new();
        tx.send(PlayerCommand::PlayPad { pad: 0, samples: clip(0.1, 100) }).unwrap();
        tx.send(PlayerCommand::PlayPad { pad: 1, samples: clip(0.2, 100) }).unwrap();
        tx.send(PlayerCommand::PadsGain(0.5)).unwrap();
        tx.send(PlayerCommand::LoadMusic { samples: clip(1.0, 3) }).unwrap();
        tx.send(PlayerCommand::MusicLoop(true)).unwrap();
        let mut out = vec![0.0; 8];
        p.render(&mut out);
        assert!(out.iter().all(|v| (v - 1.15).abs() < 1e-6), "{out:?}");
        assert!(status.music_playing.load(Ordering::Relaxed));
        let [pads_l, _] = status.pads_peak.load();
        let [music_l, _] = status.music_peak.load();
        assert!((pads_l - 0.15).abs() < 1e-6, "the pads meter sees only the pads: {pads_l}");
        assert!((music_l - 1.0).abs() < 1e-6, "the music meter sees only the music: {music_l}");
    }

    #[test]
    fn music_without_loop_stops_at_the_end() {
        let (mut p, tx, status) = Player::new();
        tx.send(PlayerCommand::LoadMusic { samples: clip(1.0, 3) }).unwrap();
        let mut out = vec![0.0; 8];
        p.render(&mut out);
        assert_eq!(out, vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0]);
        assert!(!status.music_playing.load(Ordering::Relaxed));
    }

    #[test]
    fn clip_from_pcm_keeps_first_two_of_many_channels() {
        // 4-channel interleaved PCM at the engine rate: front L/R plus two extra channels that
        // must be dropped, not mixed in.
        let frames = 100;
        let pcm: Vec<f32> = (0..frames).flat_map(|i| [i as f32, -(i as f32), 9.0, 9.0]).collect();
        let c = Clip::from_pcm("t", &pcm, 4, SAMPLE_RATE);
        assert_eq!(c.frames(), frames);
        assert_eq!(c.samples[0], 0.0);
        assert_eq!(c.samples[1], -0.0);
        assert_eq!(c.samples[2], 1.0);
        assert_eq!(c.samples[3], -1.0);
    }

    #[test]
    fn clip_from_mono_44k_is_stereo_48k() {
        let pcm: Vec<f32> = (0..44_100).map(|i| (i % 100) as f32 / 100.0).collect();
        let c = Clip::from_pcm("t", &pcm, 1, 44_100);
        assert!((c.frames() as i64 - 48_000).abs() < 4, "{}", c.frames());
        assert_eq!(c.samples[0], c.samples[1]);
    }


    #[test]
    fn empty_music_with_loop_does_not_hang_the_engine() {
        let (mut p, tx, status) = Player::new();
        tx.send(PlayerCommand::LoadMusic { samples: clip(1.0, 0) }).unwrap();
        tx.send(PlayerCommand::MusicLoop(true)).unwrap();
        let mut out = vec![0.0; 8];
        p.render(&mut out);
        assert!(out.iter().all(|v| *v == 0.0));
        assert!(!status.music_playing.load(Ordering::Relaxed));
    }

    /// The CRC-32 variant OGG pages are checksummed with: MSB-first, polynomial 0x04c11db7,
    /// initial value zero, no final XOR. `symphonia-format-ogg` rejects a page whose header CRC
    /// does not match this exactly.
    fn ogg_crc32(data: &[u8]) -> u32 {
        let mut crc: u32 = 0;
        for &byte in data {
            crc ^= (byte as u32) << 24;
            for _ in 0..8 {
                crc = if crc & 0x8000_0000 != 0 { (crc << 1) ^ 0x04c1_1db7 } else { crc << 1 };
            }
        }
        crc
    }

    /// Builds one OGG page (header, segment table, packet bodies and its CRC) holding `packets`.
    fn ogg_page(serial: u32, sequence: u32, flags: u8, absgp: u64, packets: &[&[u8]]) -> Vec<u8> {
        let mut segments = Vec::new();
        let mut body = Vec::new();
        for packet in packets {
            let mut remaining = packet.len();
            while remaining >= 255 {
                segments.push(255u8);
                remaining -= 255;
            }
            segments.push(remaining as u8);
            body.extend_from_slice(packet);
        }
        let mut page = Vec::new();
        page.extend_from_slice(b"OggS");
        page.push(0); // version
        page.push(flags);
        page.extend_from_slice(&absgp.to_le_bytes());
        page.extend_from_slice(&serial.to_le_bytes());
        page.extend_from_slice(&sequence.to_le_bytes());
        page.extend_from_slice(&[0u8; 4]); // CRC placeholder, filled in below
        page.push(segments.len() as u8);
        page.extend_from_slice(&segments);
        page.extend_from_slice(&body);
        let crc = ogg_crc32(&page);
        page[22..26].copy_from_slice(&crc.to_le_bytes());
        page
    }

    /// A minimal but valid three-page OGG Opus stream: the "OpusHead" identification packet
    /// (beginning-of-stream), an empty "OpusTags" comment packet, then one throwaway audio
    /// packet so the OGG demuxer considers the logical stream ready. `load_clip` rejects the
    /// file for its codec before any of the audio packet's bytes are decoded.
    fn minimal_ogg_opus_bytes() -> Vec<u8> {
        const SERIAL: u32 = 0x1234;
        let mut id_packet = Vec::new();
        id_packet.extend_from_slice(b"OpusHead");
        id_packet.push(1); // version
        id_packet.push(2); // channel count
        id_packet.extend_from_slice(&0u16.to_le_bytes()); // pre-skip
        id_packet.extend_from_slice(&48_000u32.to_le_bytes()); // original sample rate
        id_packet.extend_from_slice(&0u16.to_le_bytes()); // output gain
        id_packet.push(0); // channel mapping (RTP)
        assert_eq!(id_packet.len(), 19);

        let mut tags_packet = Vec::new();
        tags_packet.extend_from_slice(b"OpusTags");
        tags_packet.extend_from_slice(&0u32.to_le_bytes()); // vendor string length
        tags_packet.extend_from_slice(&0u32.to_le_bytes()); // comment count

        let audio_packet = [0x00u8];

        let mut bytes = Vec::new();
        bytes.extend(ogg_page(SERIAL, 0, 0x02, 0, &[&id_packet])); // beginning-of-stream
        bytes.extend(ogg_page(SERIAL, 1, 0x00, 0, &[&tags_packet]));
        bytes.extend(ogg_page(SERIAL, 2, 0x04, 960, &[&audio_packet])); // end-of-stream
        bytes
    }

    #[test]
    fn load_clip_rejects_opus_with_the_unsupported_message() {
        let path = std::env::temp_dir().join(format!("dhmix-test-opus-{}.opus", std::process::id()));
        std::fs::write(&path, minimal_ogg_opus_bytes()).unwrap();
        let result = load_clip(&path);
        let _ = std::fs::remove_file(&path);
        let err = result.expect_err("an Opus file must be rejected, not decoded");
        assert_eq!(err.to_string(), OPUS_UNSUPPORTED);
    }
}
