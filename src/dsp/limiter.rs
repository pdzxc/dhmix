//! Fast peak limiter for bus outputs: keeps the signal under the ceiling without hard clipping.

use super::time_coeff;
use crate::db_to_gain;

#[derive(Clone, Debug)]
pub struct Limiter {
    ceiling: f32,
    release: f32,
    gain: f32,
    pub enabled: bool,
}

impl Limiter {
    pub fn new(sample_rate: f32, ceiling_db: f32) -> Self {
        Self { ceiling: db_to_gain(ceiling_db), release: time_coeff(60.0, sample_rate), gain: 1.0, enabled: true }
    }

    pub fn process(&mut self, buf: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for frame in buf.chunks_exact_mut(2) {
            let peak = frame[0].abs().max(frame[1].abs());
            let needed = if peak * self.gain > self.ceiling { self.ceiling / peak } else { 1.0 };
            // Instant attack, smooth release.
            self.gain = if needed < self.gain { needed } else { self.release * self.gain + (1.0 - self.release) };
            frame[0] = (frame[0] * self.gain).clamp(-self.ceiling, self.ceiling);
            frame[1] = (frame[1] * self.gain).clamp(-self.ceiling, self.ceiling);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    #[test]
    fn output_never_exceeds_ceiling() {
        let mut l = Limiter::new(SAMPLE_RATE as f32, -1.0);
        let mut buf: Vec<f32> = (0..9600).map(|i| if i % 7 == 0 { 3.0 } else { 0.2 }).collect();
        l.process(&mut buf);
        let ceiling = db_to_gain(-1.0);
        assert!(buf.iter().all(|v| v.abs() <= ceiling + 1e-6));
    }
}
