//! RBJ audio-EQ-cookbook biquad filters, stereo.

use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coeffs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl Coeffs {
    pub const IDENTITY: Coeffs = Coeffs { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 };

    fn normalise(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Coeffs {
        Coeffs { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0 }
    }

    pub fn low_shelf(sample_rate: f32, freq: f32, gain_db: f32, slope: f32) -> Coeffs {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / 2.0 * ((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0).sqrt();
        let sqrt_a2alpha = 2.0 * a.sqrt() * alpha;
        Coeffs::normalise(
            a * ((a + 1.0) - (a - 1.0) * cos + sqrt_a2alpha),
            2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
            a * ((a + 1.0) - (a - 1.0) * cos - sqrt_a2alpha),
            (a + 1.0) + (a - 1.0) * cos + sqrt_a2alpha,
            -2.0 * ((a - 1.0) + (a + 1.0) * cos),
            (a + 1.0) + (a - 1.0) * cos - sqrt_a2alpha,
        )
    }

    pub fn high_shelf(sample_rate: f32, freq: f32, gain_db: f32, slope: f32) -> Coeffs {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / 2.0 * ((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0).sqrt();
        let sqrt_a2alpha = 2.0 * a.sqrt() * alpha;
        Coeffs::normalise(
            a * ((a + 1.0) + (a - 1.0) * cos + sqrt_a2alpha),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
            a * ((a + 1.0) + (a - 1.0) * cos - sqrt_a2alpha),
            (a + 1.0) - (a - 1.0) * cos + sqrt_a2alpha,
            2.0 * ((a - 1.0) - (a + 1.0) * cos),
            (a + 1.0) - (a - 1.0) * cos - sqrt_a2alpha,
        )
    }

    pub fn peaking(sample_rate: f32, freq: f32, gain_db: f32, q: f32) -> Coeffs {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        Coeffs::normalise(
            1.0 + alpha * a,
            -2.0 * cos,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * cos,
            1.0 - alpha / a,
        )
    }

    pub fn high_pass(sample_rate: f32, freq: f32, q: f32) -> Coeffs {
        let w0 = 2.0 * PI * freq / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        Coeffs::normalise(
            (1.0 + cos) / 2.0,
            -(1.0 + cos),
            (1.0 + cos) / 2.0,
            1.0 + alpha,
            -2.0 * cos,
            1.0 - alpha,
        )
    }

    pub fn low_pass(sample_rate: f32, freq: f32, q: f32) -> Coeffs {
        let w0 = 2.0 * PI * freq / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        Coeffs::normalise(
            (1.0 - cos) / 2.0,
            1.0 - cos,
            (1.0 - cos) / 2.0,
            1.0 + alpha,
            -2.0 * cos,
            1.0 - alpha,
        )
    }

    /// Magnitude response (linear) at `freq`, used by tests and the EQ display.
    pub fn magnitude_at(&self, sample_rate: f32, freq: f32) -> f32 {
        let w = 2.0 * PI * freq / sample_rate;
        let (s1, c1) = w.sin_cos();
        let (s2, c2) = (2.0 * w).sin_cos();
        let num_re = self.b0 + self.b1 * c1 + self.b2 * c2;
        let num_im = -(self.b1 * s1 + self.b2 * s2);
        let den_re = 1.0 + self.a1 * c1 + self.a2 * c2;
        let den_im = -(self.a1 * s1 + self.a2 * s2);
        (num_re * num_re + num_im * num_im).sqrt() / (den_re * den_re + den_im * den_im).sqrt()
    }
}

/// Transposed direct-form-II biquad running on interleaved stereo.
#[derive(Clone, Debug)]
pub struct StereoBiquad {
    coeffs: Coeffs,
    z1: [f32; 2],
    z2: [f32; 2],
}

impl Default for StereoBiquad {
    fn default() -> Self {
        Self::new(Coeffs::IDENTITY)
    }
}

impl StereoBiquad {
    pub fn new(coeffs: Coeffs) -> Self {
        Self { coeffs, z1: [0.0; 2], z2: [0.0; 2] }
    }

    pub fn set_coeffs(&mut self, coeffs: Coeffs) {
        self.coeffs = coeffs;
    }

    pub fn reset(&mut self) {
        self.z1 = [0.0; 2];
        self.z2 = [0.0; 2];
    }

    pub fn process(&mut self, buf: &mut [f32]) {
        let c = self.coeffs;
        for frame in buf.chunks_exact_mut(2) {
            for (ch, x) in frame.iter_mut().enumerate() {
                let input = *x;
                let y = c.b0 * input + self.z1[ch];
                self.z1[ch] = c.b1 * input - c.a1 * y + self.z2[ch];
                self.z2[ch] = c.b2 * input - c.a2 * y;
                *x = y;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    fn measure_gain_db(coeffs: Coeffs, freq: f32) -> f32 {
        // Run a real sine through the filter and measure its steady-state peak.
        let sr = SAMPLE_RATE as f32;
        let mut filter = StereoBiquad::new(coeffs);
        let n = 48_000;
        let mut buf: Vec<f32> = (0..n)
            .flat_map(|i| {
                let s = (2.0 * PI * freq * i as f32 / sr).sin();
                [s, s]
            })
            .collect();
        filter.process(&mut buf);
        let peak = buf[n..].iter().fold(0f32, |m, v| m.max(v.abs()));
        crate::gain_to_db(peak)
    }

    #[test]
    fn low_shelf_boosts_bass_and_leaves_treble_alone() {
        let c = Coeffs::low_shelf(SAMPLE_RATE as f32, 100.0, 6.0, 0.9);
        assert!((measure_gain_db(c, 30.0) - 6.0).abs() < 0.5, "bass should be +6 dB");
        assert!(measure_gain_db(c, 5000.0).abs() < 0.2, "treble should be untouched");
    }

    #[test]
    fn high_shelf_cuts_treble_and_leaves_bass_alone() {
        let c = Coeffs::high_shelf(SAMPLE_RATE as f32, 8000.0, -6.0, 0.9);
        assert!((measure_gain_db(c, 16000.0) + 6.0).abs() < 0.5);
        assert!(measure_gain_db(c, 200.0).abs() < 0.2);
    }

    #[test]
    fn high_pass_removes_rumble() {
        let c = Coeffs::high_pass(SAMPLE_RATE as f32, 80.0, 0.707);
        assert!(measure_gain_db(c, 20.0) < -20.0);
        assert!(measure_gain_db(c, 1000.0).abs() < 0.2);
    }

    #[test]
    fn magnitude_response_matches_measured_output() {
        let c = Coeffs::peaking(SAMPLE_RATE as f32, 1000.0, 4.0, 1.0);
        let analytic = crate::gain_to_db(c.magnitude_at(SAMPLE_RATE as f32, 1000.0));
        assert!((analytic - 4.0).abs() < 0.05);
        assert!((measure_gain_db(c, 1000.0) - analytic).abs() < 0.2);
    }
}
