//! Feed-forward peak compressor with stereo-linked gain reduction and makeup gain.

use super::time_coeff;
use crate::{db_to_gain, gain_to_db};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressorSettings {
    pub enabled: bool,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub makeup_db: f32,
}

impl Default for CompressorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -18.0,
            ratio: 3.0,
            attack_ms: 5.0,
            release_ms: 80.0,
            makeup_db: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Compressor {
    sample_rate: f32,
    settings: CompressorSettings,
    attack: f32,
    release: f32,
    envelope: f32,
    /// Current gain reduction in dB (positive number), for the UI meter.
    pub reduction_db: f32,
}

impl Compressor {
    pub fn new(sample_rate: f32) -> Self {
        let mut c = Self {
            sample_rate,
            settings: CompressorSettings::default(),
            attack: 0.0,
            release: 0.0,
            envelope: 0.0,
            reduction_db: 0.0,
        };
        c.apply(CompressorSettings::default());
        c
    }

    pub fn apply(&mut self, s: CompressorSettings) {
        // A ratio under 1 would divide the overshoot by zero or invert it; presets are user-editable.
        self.settings = CompressorSettings { ratio: s.ratio.clamp(1.0, 20.0), ..s };
        self.attack = time_coeff(s.attack_ms, self.sample_rate);
        self.release = time_coeff(s.release_ms, self.sample_rate);
    }

    pub fn process(&mut self, buf: &mut [f32]) {
        if !self.settings.enabled {
            self.reduction_db = 0.0;
            return;
        }
        let s = self.settings;
        let makeup = db_to_gain(s.makeup_db);
        let mut max_reduction = 0.0f32;
        for frame in buf.chunks_exact_mut(2) {
            let peak = frame[0].abs().max(frame[1].abs());
            let coeff = if peak > self.envelope { self.attack } else { self.release };
            self.envelope = coeff * self.envelope + (1.0 - coeff) * peak;
            let level_db = gain_to_db(self.envelope);
            let over = level_db - s.threshold_db;
            let reduction = if over > 0.0 { over - over / s.ratio } else { 0.0 };
            max_reduction = max_reduction.max(reduction);
            let gain = db_to_gain(-reduction) * makeup;
            frame[0] *= gain;
            frame[1] *= gain;
        }
        self.reduction_db = max_reduction;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    fn loud_block(db: f32) -> Vec<f32> {
        vec![db_to_gain(db); 2 * 48_000]
    }

    #[test]
    fn signal_above_threshold_is_reduced_by_ratio() {
        let mut c = Compressor::new(SAMPLE_RATE as f32);
        c.apply(CompressorSettings {
            enabled: true,
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 50.0,
            makeup_db: 0.0,
        });
        // -8 dB input is 12 dB over the threshold; at 4:1 it should come out 9 dB lower.
        let mut buf = loud_block(-8.0);
        c.process(&mut buf);
        let out_db = gain_to_db(buf[buf.len() - 1].abs());
        assert!((out_db - (-17.0)).abs() < 0.3, "got {out_db} dB");
        assert!((c.reduction_db - 9.0).abs() < 0.3);
    }

    #[test]
    fn signal_below_threshold_passes_untouched() {
        let mut c = Compressor::new(SAMPLE_RATE as f32);
        c.apply(CompressorSettings { enabled: true, threshold_db: -20.0, ..Default::default() });
        let mut buf = loud_block(-30.0);
        c.process(&mut buf);
        assert!((gain_to_db(buf[buf.len() - 1].abs()) - (-30.0)).abs() < 0.01);
    }


    #[test]
    fn ratio_below_one_from_a_hand_edited_preset_is_clamped_to_no_compression() {
        let mut c = Compressor::new(SAMPLE_RATE as f32);
        c.apply(CompressorSettings { enabled: true, threshold_db: -20.0, ratio: 0.0, ..Default::default() });
        let mut buf = loud_block(-8.0);
        c.process(&mut buf);
        assert!(buf.iter().all(|v| v.is_finite()));
        assert!((gain_to_db(buf[buf.len() - 1].abs()) - (-8.0)).abs() < 0.01, "ratio 1:1 leaves level alone");
    }
}
