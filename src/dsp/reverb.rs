//! Compact Freeverb-style reverb (parallel combs + series allpasses per channel).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReverbSettings {
    pub enabled: bool,
    /// 0..1 room size.
    pub size: f32,
    /// 0..1 high-frequency damping.
    pub damping: f32,
    /// 0..1 wet share in the output.
    pub mix: f32,
}

impl Default for ReverbSettings {
    fn default() -> Self {
        Self { enabled: false, size: 0.5, damping: 0.5, mix: 0.25 }
    }
}

const COMB_TUNING: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;
const TUNING_RATE: f32 = 44_100.0;

#[derive(Clone, Debug)]
struct Comb {
    buf: Vec<f32>,
    idx: usize,
    filter_store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self { buf: vec![0.0; len.max(1)], idx: 0, filter_store: 0.0 }
    }
    fn process(&mut self, input: f32, feedback: f32, damp: f32) -> f32 {
        let out = self.buf[self.idx];
        self.filter_store = out * (1.0 - damp) + self.filter_store * damp;
        self.buf[self.idx] = input + self.filter_store * feedback;
        self.idx = (self.idx + 1) % self.buf.len();
        out
    }
}

#[derive(Clone, Debug)]
struct Allpass {
    buf: Vec<f32>,
    idx: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self { buf: vec![0.0; len.max(1)], idx: 0 }
    }
    fn process(&mut self, input: f32) -> f32 {
        let bufout = self.buf[self.idx];
        let out = -input + bufout;
        self.buf[self.idx] = input + bufout * 0.5;
        self.idx = (self.idx + 1) % self.buf.len();
        out
    }
}

#[derive(Clone, Debug)]
pub struct Reverb {
    settings: ReverbSettings,
    combs: [Vec<Comb>; 2],
    allpasses: [Vec<Allpass>; 2],
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let scale = sample_rate / TUNING_RATE;
        let make_combs = |offset: usize| {
            COMB_TUNING.iter().map(|&t| Comb::new(((t + offset) as f32 * scale) as usize)).collect()
        };
        let make_allpasses = |offset: usize| {
            ALLPASS_TUNING.iter().map(|&t| Allpass::new(((t + offset) as f32 * scale) as usize)).collect()
        };
        Self {
            settings: ReverbSettings::default(),
            combs: [make_combs(0), make_combs(STEREO_SPREAD)],
            allpasses: [make_allpasses(0), make_allpasses(STEREO_SPREAD)],
        }
    }

    pub fn apply(&mut self, s: ReverbSettings) {
        self.settings = ReverbSettings {
            enabled: s.enabled,
            size: s.size.clamp(0.0, 1.0),
            damping: s.damping.clamp(0.0, 1.0),
            mix: s.mix.clamp(0.0, 1.0),
        };
    }

    pub fn process(&mut self, buf: &mut [f32]) {
        if !self.settings.enabled {
            return;
        }
        let s = self.settings;
        let feedback = 0.7 + s.size * 0.28;
        let damp = s.damping * 0.4;
        let wet_gain = 0.015;
        for frame in buf.chunks_exact_mut(2) {
            let input = (frame[0] + frame[1]) * 0.5 * wet_gain;
            for ((sample, combs), allpasses) in frame.iter_mut().zip(&mut self.combs).zip(&mut self.allpasses) {
                let mut wet = 0.0;
                for comb in combs.iter_mut() {
                    wet += comb.process(input, feedback, damp);
                }
                for ap in allpasses.iter_mut() {
                    wet = ap.process(wet);
                }
                *sample = *sample * (1.0 - s.mix) + wet * s.mix;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    #[test]
    fn reverb_produces_a_decaying_tail_after_an_impulse() {
        let mut r = Reverb::new(SAMPLE_RATE as f32);
        r.apply(ReverbSettings { enabled: true, size: 0.6, damping: 0.5, mix: 1.0 });
        let mut buf = vec![0.0f32; 2 * 48_000 * 2];
        buf[0] = 1.0;
        buf[1] = 1.0;
        r.process(&mut buf);
        let energy = |range: std::ops::Range<usize>| buf[range].iter().map(|v| v * v).sum::<f32>();
        let early = energy(2 * 4800..2 * 9600);
        let late = energy(2 * 72_000..2 * 76_800);
        assert!(early > 0.0, "tail exists");
        assert!(late < early, "tail decays");
        assert!(buf.iter().all(|v| v.is_finite() && v.abs() < 2.0), "stable");
    }
}
