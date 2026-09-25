//! Everything the UI can change and the engine reads once per block. All fields are `Copy`, so
//! the engine can take a cheap snapshot under an uncontended `try_lock`.

use crate::dsp::compressor::CompressorSettings;
use crate::dsp::echo::EchoSettings;
use crate::dsp::eq::EqSettings;
use crate::dsp::gate::GateSettings;
use crate::dsp::meter::AtomicPeak;
use crate::dsp::reverb::ReverbSettings;
use crate::{NUM_BUSES, NUM_HW_BUSES, NUM_STRIPS, PLAYER_STRIP};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StripSettings {
    pub gain_db: f32,
    pub mute: bool,
    pub solo: bool,
    pub mono: bool,
    /// -1 (left) .. +1 (right).
    pub pan: f32,
    pub routing: [bool; NUM_BUSES],
    pub denoise: bool,
    pub gate: GateSettings,
    pub eq: EqSettings,
    pub comp: CompressorSettings,
    pub echo: EchoSettings,
    pub reverb: ReverbSettings,
}

impl Default for StripSettings {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            mute: false,
            solo: false,
            mono: false,
            pan: 0.0,
            routing: [false; NUM_BUSES],
            denoise: false,
            gate: GateSettings::default(),
            eq: EqSettings::default(),
            comp: CompressorSettings::default(),
            echo: EchoSettings::default(),
            reverb: ReverbSettings::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BusSettings {
    pub gain_db: f32,
    pub mute: bool,
    pub bass_db: f32,
    pub treble_db: f32,
    /// Limiter ceiling in dB: peaks above it are held down. Presets saved before the ceiling
    /// was adjustable carry no value and get the default.
    #[serde(default = "default_limit_db")]
    pub limit_db: f32,
}

impl Default for BusSettings {
    fn default() -> Self {
        Self { gain_db: 0.0, mute: false, bass_db: 0.0, treble_db: 0.0, limit_db: LIMIT_DEFAULT_DB }
    }
}

fn default_limit_db() -> f32 {
    LIMIT_DEFAULT_DB
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MixSettings {
    pub strips: [StripSettings; NUM_STRIPS],
    pub buses: [BusSettings; NUM_BUSES],
}

impl Default for MixSettings {
    /// A streaming-friendly starting point: everything to A1 (headphones), mics and player
    /// also to B1 (the virtual cable that OBS or Discord picks up as a microphone).
    fn default() -> Self {
        let mut strips = [StripSettings::default(); NUM_STRIPS];
        for (i, strip) in strips.iter_mut().enumerate() {
            strip.routing[0] = true;
            if i == 0 || i == PLAYER_STRIP {
                strip.routing[NUM_HW_BUSES] = true;
            }
        }
        Self { strips, buses: [BusSettings::default(); NUM_BUSES] }
    }
}

pub const GAIN_DB_RANGE: std::ops::RangeInclusive<f32> = -60.0..=12.0;
pub const RATIO_RANGE: std::ops::RangeInclusive<f32> = 1.0..=20.0;
/// Bus limiter ceiling: 0 dB only stops clipping; lower values tame a loud output.
pub const LIMIT_DB_RANGE: std::ops::RangeInclusive<f32> = -40.0..=0.0;
/// Just under full scale, so inter-sample peaks do not clip the converter.
pub const LIMIT_DEFAULT_DB: f32 = -0.5;

impl MixSettings {
    pub fn any_solo(&self) -> bool {
        self.strips.iter().any(|s| s.solo)
    }

    /// Pulls every value back into the range the UI offers, so a hand-edited preset cannot
    /// drive the engine with inverted ratios or absurd gains.
    pub fn clamped(mut self) -> Self {
        for s in self.strips.iter_mut() {
            s.gain_db = s.gain_db.clamp(*GAIN_DB_RANGE.start(), *GAIN_DB_RANGE.end());
            s.pan = s.pan.clamp(-1.0, 1.0);
            s.comp.ratio = s.comp.ratio.clamp(*RATIO_RANGE.start(), *RATIO_RANGE.end());
            s.comp.makeup_db = s.comp.makeup_db.clamp(0.0, 24.0);
        }
        for b in self.buses.iter_mut() {
            b.gain_db = b.gain_db.clamp(*GAIN_DB_RANGE.start(), *GAIN_DB_RANGE.end());
            b.limit_db = b.limit_db.clamp(*LIMIT_DB_RANGE.start(), *LIMIT_DB_RANGE.end());
        }
        self
    }
}

/// Levels published by the engine for the UI, one writer and many readers, no locks.
#[derive(Debug, Default)]
pub struct Meters {
    pub strips: [AtomicPeak; NUM_STRIPS],
    pub buses: [AtomicPeak; NUM_BUSES],
    pub comp_reduction_db: [AtomicU32; NUM_STRIPS],
    pub gate_open: [AtomicBool; NUM_STRIPS],
    pub input_connected: [AtomicBool; NUM_STRIPS],
    pub output_connected: [AtomicBool; NUM_BUSES],
    /// Blocks where an input device had not delivered enough audio.
    pub underruns: AtomicU32,
    /// Blocks dropped because an output device had not consumed the previous ones.
    pub overruns: AtomicU32,
}

impl Meters {
    pub fn set_reduction(&self, strip: usize, db: f32) {
        self.comp_reduction_db[strip].store(db.to_bits(), Ordering::Relaxed);
    }
    pub fn reduction(&self, strip: usize) -> f32 {
        f32::from_bits(self.comp_reduction_db[strip].load(Ordering::Relaxed))
    }
}
