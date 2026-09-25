//! Noise gate with attack, hold and release, stereo linked.

use super::time_coeff;
use crate::db_to_gain;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GateSettings {
    pub enabled: bool,
    pub threshold_db: f32,
    pub attack_ms: f32,
    pub hold_ms: f32,
    pub release_ms: f32,
}

impl Default for GateSettings {
    fn default() -> Self {
        Self { enabled: false, threshold_db: -45.0, attack_ms: 2.0, hold_ms: 100.0, release_ms: 120.0 }
    }
}

#[derive(Clone, Debug)]
pub struct Gate {
    sample_rate: f32,
    settings: GateSettings,
    attack: f32,
    release: f32,
    hold_samples: u32,
    hold_counter: u32,
    gain: f32,
    /// True while the gate is letting signal through, for the UI indicator.
    pub open: bool,
}

impl Gate {
    pub fn new(sample_rate: f32) -> Self {
        let mut g = Self {
            sample_rate,
            settings: GateSettings::default(),
            attack: 0.0,
            release: 0.0,
            hold_samples: 0,
            hold_counter: 0,
            gain: 1.0,
            open: true,
        };
        g.apply(GateSettings::default());
        g
    }

    pub fn apply(&mut self, s: GateSettings) {
        self.settings = s;
        self.attack = time_coeff(s.attack_ms, self.sample_rate);
        self.release = time_coeff(s.release_ms, self.sample_rate);
        self.hold_samples = (s.hold_ms * 0.001 * self.sample_rate) as u32;
    }

    pub fn process(&mut self, buf: &mut [f32]) {
        if !self.settings.enabled {
            self.open = true;
            self.gain = 1.0;
            return;
        }
        let threshold = db_to_gain(self.settings.threshold_db);
        for frame in buf.chunks_exact_mut(2) {
            let peak = frame[0].abs().max(frame[1].abs());
            if peak >= threshold {
                self.open = true;
                self.hold_counter = self.hold_samples;
            } else if self.hold_counter > 0 {
                self.hold_counter -= 1;
            } else {
                self.open = false;
            }
            let target = if self.open { 1.0 } else { 0.0 };
            let coeff = if target > self.gain { self.attack } else { self.release };
            self.gain = coeff * self.gain + (1.0 - coeff) * target;
            frame[0] *= self.gain;
            frame[1] *= self.gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    #[test]
    fn gate_silences_quiet_signal_after_hold_and_release() {
        let mut g = Gate::new(SAMPLE_RATE as f32);
        g.apply(GateSettings { enabled: true, threshold_db: -40.0, attack_ms: 1.0, hold_ms: 10.0, release_ms: 10.0 });
        let mut buf = vec![db_to_gain(-60.0); 2 * 48_000];
        g.process(&mut buf);
        assert!(!g.open);
        assert!(buf[buf.len() - 1].abs() < 1e-6, "tail should be silent");
    }

    #[test]
    fn gate_stays_open_during_hold_then_closes() {
        let mut g = Gate::new(SAMPLE_RATE as f32);
        g.apply(GateSettings { enabled: true, threshold_db: -40.0, attack_ms: 0.001, hold_ms: 5.0, release_ms: 0.001 });
        let mut loud = vec![db_to_gain(-10.0); 2];
        g.process(&mut loud);
        assert!(g.open, "loud frame opens the gate");

        let hold_samples = (5.0 * 0.001 * SAMPLE_RATE as f32) as usize;
        let mut quiet = vec![db_to_gain(-60.0); hold_samples * 2];
        g.process(&mut quiet);
        assert!(g.open, "gate should still be open for the whole hold window");

        let mut one_more = vec![db_to_gain(-60.0); 2];
        g.process(&mut one_more);
        assert!(!g.open, "gate closes once the hold window elapses");
    }

    #[test]
    fn gate_opens_for_loud_signal() {
        let mut g = Gate::new(SAMPLE_RATE as f32);
        g.apply(GateSettings { enabled: true, threshold_db: -40.0, ..Default::default() });
        let mut buf = vec![db_to_gain(-20.0); 2 * 4800];
        g.process(&mut buf);
        assert!(g.open);
        assert!((buf[buf.len() - 1] - db_to_gain(-20.0)).abs() < 1e-3);
    }
}
