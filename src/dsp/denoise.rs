//! RNNoise-based denoiser (via the pure-Rust `nnnoiseless` port). Works on 480-frame blocks at 48 kHz.

use nnnoiseless::DenoiseState;

pub const FRAME: usize = DenoiseState::FRAME_SIZE;
/// RNNoise was trained on 16-bit-scaled samples.
const SCALE: f32 = 32_768.0;

pub struct Denoiser {
    states: [Box<DenoiseState<'static>>; 2],
    scratch_in: [f32; FRAME],
    scratch_out: [f32; FRAME],
    /// Voice-activity probability from the last frame (0..1), for the UI.
    pub vad: f32,
    pub enabled: bool,
}

impl Default for Denoiser {
    fn default() -> Self {
        Self::new()
    }
}

impl Denoiser {
    pub fn new() -> Self {
        Self {
            states: [DenoiseState::new(), DenoiseState::new()],
            scratch_in: [0.0; FRAME],
            scratch_out: [0.0; FRAME],
            vad: 0.0,
            enabled: false,
        }
    }

    /// Processes interleaved stereo in place. `buf` must hold whole 480-frame blocks.
    pub fn process(&mut self, buf: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for block in buf.chunks_exact_mut(FRAME * 2) {
            let mut vad = 0.0f32;
            for ch in 0..2 {
                for (i, v) in self.scratch_in.iter_mut().enumerate() {
                    *v = block[i * 2 + ch] * SCALE;
                }
                vad = vad.max(self.states[ch].process_frame(&mut self.scratch_out, &self.scratch_in));
                for (i, v) in self.scratch_out.iter().enumerate() {
                    block[i * 2 + ch] = v / SCALE;
                }
            }
            self.vad = vad;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denoiser_attenuates_white_noise() {
        let mut d = Denoiser::new();
        d.enabled = true;
        // Deterministic pseudo-random noise.
        let mut seed: u32 = 12345;
        let mut next = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        let frames = FRAME * 100;
        let mut buf: Vec<f32> = (0..frames * 2).map(|_| next() * 0.05).collect();
        let before: f32 = buf.iter().map(|v| v * v).sum::<f32>();
        d.process(&mut buf);
        let after: f32 = buf[frames..].iter().map(|v| v * v).sum::<f32>();
        assert!(after < before * 0.5, "noise energy should drop: before {before} after {after}");
    }

    #[test]
    fn disabled_denoiser_is_a_bypass() {
        let mut d = Denoiser::new();
        let mut buf = vec![0.25f32; FRAME * 2];
        d.process(&mut buf);
        assert!(buf.iter().all(|v| *v == 0.25));
    }
}
