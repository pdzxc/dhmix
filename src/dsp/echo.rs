//! Feedback echo (delay line) with wet/dry mix.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EchoSettings {
    pub enabled: bool,
    pub time_ms: f32,
    /// 0..0.95, how much of the echo feeds back into itself.
    pub feedback: f32,
    /// 0..1, wet share in the output.
    pub mix: f32,
}

impl Default for EchoSettings {
    fn default() -> Self {
        Self { enabled: false, time_ms: 250.0, feedback: 0.35, mix: 0.3 }
    }
}

pub const MAX_ECHO_MS: f32 = 2000.0;

#[derive(Clone, Debug)]
pub struct Echo {
    sample_rate: f32,
    settings: EchoSettings,
    line: Vec<[f32; 2]>,
    write: usize,
}

impl Echo {
    pub fn new(sample_rate: f32) -> Self {
        let len = (MAX_ECHO_MS * 0.001 * sample_rate) as usize + 1;
        Self { sample_rate, settings: EchoSettings::default(), line: vec![[0.0; 2]; len], write: 0 }
    }

    pub fn apply(&mut self, s: EchoSettings) {
        self.settings = s;
        self.settings.time_ms = s.time_ms.clamp(1.0, MAX_ECHO_MS);
        self.settings.feedback = s.feedback.clamp(0.0, 0.95);
        self.settings.mix = s.mix.clamp(0.0, 1.0);
    }

    pub fn process(&mut self, buf: &mut [f32]) {
        if !self.settings.enabled {
            return;
        }
        let s = self.settings;
        let delay = ((s.time_ms * 0.001 * self.sample_rate) as usize).clamp(1, self.line.len() - 1);
        let len = self.line.len();
        for frame in buf.chunks_exact_mut(2) {
            let read = (self.write + len - delay) % len;
            let delayed = self.line[read];
            let dry = [frame[0], frame[1]];
            self.line[self.write] = [dry[0] + delayed[0] * s.feedback, dry[1] + delayed[1] * s.feedback];
            self.write = (self.write + 1) % len;
            frame[0] = dry[0] * (1.0 - s.mix) + delayed[0] * s.mix;
            frame[1] = dry[1] * (1.0 - s.mix) + delayed[1] * s.mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    #[test]
    fn disabled_echo_is_a_bypass() {
        let mut e = Echo::new(SAMPLE_RATE as f32);
        e.apply(EchoSettings { enabled: false, time_ms: 100.0, feedback: 0.5, mix: 0.5 });
        let mut buf = vec![0.3, -0.4, 0.1, 0.2, -0.6, 0.7];
        let original = buf.clone();
        e.process(&mut buf);
        assert_eq!(buf, original, "disabled echo must pass audio through unchanged");
    }

    #[test]
    fn impulse_repeats_after_the_delay_time() {
        let mut e = Echo::new(SAMPLE_RATE as f32);
        e.apply(EchoSettings { enabled: true, time_ms: 100.0, feedback: 0.5, mix: 0.5 });
        let frames = 48_000 / 2;
        let mut buf = vec![0.0f32; frames * 2];
        buf[0] = 1.0;
        buf[1] = 1.0;
        e.process(&mut buf);
        let delay_frames = 4800;
        assert!((buf[0] - 0.5).abs() < 1e-6, "dry impulse at half mix");
        assert!((buf[delay_frames * 2] - 0.5).abs() < 1e-6, "first echo");
        assert!((buf[delay_frames * 4] - 0.25).abs() < 1e-6, "second echo at feedback 0.5");
    }
}
