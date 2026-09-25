//! Soundboard and music player that feeds the PLAYER strip. Pads are fire-and-forget voices that
//! may overlap; the music track is a single voice with pause, seek and loop.

use crate::audio::resample::LinearResampler;
use crate::SAMPLE_RATE;
use anyhow::{anyhow, Context, Result};
use crossbeam_channel::{Receiver, Sender};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
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
    PlayPad { pad: usize, samples: Arc<Vec<f32>>, gain: f32 },
    StopPads,
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
}

struct PadVoice {
    pad: usize,
    samples: Arc<Vec<f32>>,
    pos: usize,
    gain: f32,
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
}

impl Player {
    pub fn new() -> (Self, Sender<PlayerCommand>, Arc<PlayerStatus>) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let status = Arc::new(PlayerStatus::default());
        let player = Self { rx, status: status.clone(), pads: Vec::new(), music: None, music_loop: false, music_gain: 1.0 };
        (player, tx, status)
    }

    fn handle(&mut self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::PlayPad { pad, samples, gain } => {
                self.pads.retain(|v| v.pad != pad);
                self.pads.push(PadVoice { pad, samples, pos: 0, gain });
            }
            PlayerCommand::StopPads => self.pads.clear(),
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

    /// Writes one block of mixed pads + music into `out` (interleaved stereo, overwritten).
    pub fn render(&mut self, out: &mut [f32]) {
        while let Ok(cmd) = self.rx.try_recv() {
            self.handle(cmd);
        }
        out.fill(0.0);
        for voice in &mut self.pads {
            let end = (voice.pos + out.len()).min(voice.samples.len());
            for (o, s) in out.iter_mut().zip(voice.samples[voice.pos..end].iter()) {
                *o += s * voice.gain;
            }
            voice.pos = end;
        }
        self.pads.retain(|v| v.pos < v.samples.len());

        if let Some(m) = &mut self.music {
            if m.playing {
                let mut written = 0;
                while written < out.len() && !m.samples.is_empty() {
                    let end = (m.pos + out.len() - written).min(m.samples.len());
                    for (o, s) in out[written..].iter_mut().zip(m.samples[m.pos..end].iter()) {
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
        tx.send(PlayerCommand::PlayPad { pad: 0, samples: clip(0.5, 6), gain: 1.0 }).unwrap();
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
        tx.send(PlayerCommand::PlayPad { pad: 0, samples: clip(0.1, 100), gain: 1.0 }).unwrap();
        tx.send(PlayerCommand::PlayPad { pad: 1, samples: clip(0.2, 100), gain: 0.5 }).unwrap();
        tx.send(PlayerCommand::LoadMusic { samples: clip(1.0, 3) }).unwrap();
        tx.send(PlayerCommand::MusicLoop(true)).unwrap();
        let mut out = vec![0.0; 8];
        p.render(&mut out);
        assert!(out.iter().all(|v| (v - 1.2).abs() < 1e-6), "{out:?}");
        assert!(status.music_playing.load(Ordering::Relaxed));
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
}
