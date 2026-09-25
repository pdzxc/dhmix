//! Four-band voice EQ: low cut, bass shelf, mid peak, treble shelf.

use super::biquad::{Coeffs, StereoBiquad};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EqSettings {
    pub enabled: bool,
    /// Removes rumble below 80 Hz.
    pub low_cut: bool,
    pub bass_db: f32,
    pub mid_db: f32,
    pub treble_db: f32,
}

impl Default for EqSettings {
    fn default() -> Self {
        Self { enabled: false, low_cut: false, bass_db: 0.0, mid_db: 0.0, treble_db: 0.0 }
    }
}

pub const BASS_HZ: f32 = 100.0;
pub const MID_HZ: f32 = 1000.0;
pub const TREBLE_HZ: f32 = 8000.0;
pub const LOW_CUT_HZ: f32 = 80.0;

#[derive(Clone, Debug)]
pub struct Eq {
    sample_rate: f32,
    settings: EqSettings,
    low_cut: StereoBiquad,
    bass: StereoBiquad,
    mid: StereoBiquad,
    treble: StereoBiquad,
}

impl Eq {
    pub fn new(sample_rate: f32) -> Self {
        let mut eq = Self {
            sample_rate,
            settings: EqSettings::default(),
            low_cut: StereoBiquad::default(),
            bass: StereoBiquad::default(),
            mid: StereoBiquad::default(),
            treble: StereoBiquad::default(),
        };
        eq.recompute();
        eq
    }

    /// Recomputes coefficients only when a value actually changed.
    pub fn apply(&mut self, s: EqSettings) {
        if s == self.settings {
            return;
        }
        self.settings = s;
        self.recompute();
    }

    fn recompute(&mut self) {
        let s = self.settings;
        let sr = self.sample_rate;
        self.low_cut.set_coeffs(Coeffs::high_pass(sr, LOW_CUT_HZ, 0.707));
        self.bass.set_coeffs(Coeffs::low_shelf(sr, BASS_HZ, s.bass_db, 0.9));
        self.mid.set_coeffs(Coeffs::peaking(sr, MID_HZ, s.mid_db, 1.0));
        self.treble.set_coeffs(Coeffs::high_shelf(sr, TREBLE_HZ, s.treble_db, 0.9));
    }

    pub fn process(&mut self, buf: &mut [f32]) {
        if !self.settings.enabled {
            return;
        }
        if self.settings.low_cut {
            self.low_cut.process(buf);
        }
        if self.settings.bass_db != 0.0 {
            self.bass.process(buf);
        }
        if self.settings.mid_db != 0.0 {
            self.mid.process(buf);
        }
        if self.settings.treble_db != 0.0 {
            self.treble.process(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;
    use std::f32::consts::PI;

    fn sine_peak_after(eq: &mut Eq, freq: f32) -> f32 {
        let sr = SAMPLE_RATE as f32;
        let n = 48_000;
        let mut buf: Vec<f32> = (0..n)
            .flat_map(|i| {
                let s = (2.0 * PI * freq * i as f32 / sr).sin();
                [s, s]
            })
            .collect();
        eq.process(&mut buf);
        crate::gain_to_db(buf[n..].iter().fold(0f32, |m, v| m.max(v.abs())))
    }

    #[test]
    fn disabled_eq_is_a_bypass() {
        let mut eq = Eq::new(SAMPLE_RATE as f32);
        eq.apply(EqSettings { enabled: false, bass_db: 12.0, ..Default::default() });
        assert!(sine_peak_after(&mut eq, 50.0).abs() < 0.01);
    }

    #[test]
    fn bass_boost_raises_low_frequencies() {
        let mut eq = Eq::new(SAMPLE_RATE as f32);
        eq.apply(EqSettings { enabled: true, bass_db: 6.0, ..Default::default() });
        assert!((sine_peak_after(&mut eq, 40.0) - 6.0).abs() < 0.5);
        let mut eq2 = Eq::new(SAMPLE_RATE as f32);
        eq2.apply(EqSettings { enabled: true, bass_db: 6.0, ..Default::default() });
        assert!(sine_peak_after(&mut eq2, 4000.0).abs() < 0.3);
    }
}
