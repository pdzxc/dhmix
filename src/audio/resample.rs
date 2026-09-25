//! Linear-interpolation stereo resampler with an internal FIFO, good enough to bridge a
//! 44.1 kHz device to the 48 kHz engine.

#[derive(Clone, Debug)]
pub struct LinearResampler {
    step: f64,
    pos: f64,
    /// Interleaved stereo frames waiting to be consumed.
    fifo: Vec<f32>,
}

impl LinearResampler {
    pub fn new(src_rate: u32, dst_rate: u32) -> Self {
        Self { step: src_rate as f64 / dst_rate as f64, pos: 0.0, fifo: Vec::with_capacity(8192) }
    }

    pub fn is_identity(&self) -> bool {
        self.step == 1.0
    }

    pub fn push(&mut self, stereo: &[f32]) {
        self.fifo.extend_from_slice(stereo);
    }

    fn frames_buffered(&self) -> usize {
        self.fifo.len() / 2
    }

    /// True when frames `floor(pos + step*(n-1))` and the one after it are both buffered.
    fn can_produce(&self, n: usize) -> bool {
        n == 0 || (self.pos + self.step * (n as f64 - 1.0)).floor() as usize + 2 <= self.frames_buffered()
    }

    /// Output frames that can be produced from what is buffered right now.
    pub fn available_out(&self) -> usize {
        let frames = self.frames_buffered() as f64;
        if frames < self.pos + 2.0 {
            return 0;
        }
        // Closed form, then nudged so it agrees exactly with `can_produce` despite rounding.
        let mut n = (((frames - 1.0 - self.pos) / self.step + 1.0).ceil() as usize).saturating_sub(1);
        while n > 0 && !self.can_produce(n) {
            n -= 1;
        }
        while self.can_produce(n + 1) {
            n += 1;
        }
        n
    }

    /// Extra source frames that must be pushed before `pull` can fill `out_frames`.
    pub fn needed_input(&self, out_frames: usize) -> usize {
        if out_frames == 0 {
            return 0;
        }
        let last = self.pos + self.step * (out_frames as f64 - 1.0);
        let required = last.floor() as usize + 2;
        required.saturating_sub(self.frames_buffered())
    }

    /// Fills `out` (interleaved stereo). Returns false and leaves `out` untouched if there is
    /// not enough buffered input.
    pub fn pull(&mut self, out: &mut [f32]) -> bool {
        let out_frames = out.len() / 2;
        if !self.can_produce(out_frames) {
            return false;
        }
        for frame in out.chunks_exact_mut(2) {
            let i = self.pos.floor() as usize;
            let t = (self.pos - i as f64) as f32;
            for (ch, out_sample) in frame.iter_mut().enumerate() {
                let a = self.fifo[i * 2 + ch];
                let b = self.fifo[(i + 1) * 2 + ch];
                *out_sample = a + (b - a) * t;
            }
            self.pos += self.step;
        }
        let consumed = self.pos.floor() as usize;
        self.fifo.drain(..consumed * 2);
        self.pos -= consumed as f64;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(frames: usize) -> Vec<f32> {
        (0..frames).flat_map(|i| [i as f32, -(i as f32)]).collect()
    }

    #[test]
    fn identity_rate_passes_samples_through() {
        let mut r = LinearResampler::new(48_000, 48_000);
        r.push(&ramp(10));
        let mut out = vec![0.0; 16];
        assert!(r.pull(&mut out));
        assert_eq!(&out[..6], &[0.0, 0.0, 1.0, -1.0, 2.0, -2.0]);
    }

    #[test]
    fn upsampling_interpolates_between_frames() {
        let mut r = LinearResampler::new(24_000, 48_000);
        r.push(&ramp(10));
        let mut out = vec![0.0; 8];
        assert!(r.pull(&mut out));
        assert_eq!(&out[..8], &[0.0, 0.0, 0.5, -0.5, 1.0, -1.0, 1.5, -1.5]);
    }

    #[test]
    fn needed_input_is_exactly_enough_to_pull() {
        let mut r = LinearResampler::new(44_100, 48_000);
        let out_frames = 480;
        let mut produced = 0;
        for _ in 0..50 {
            let need = r.needed_input(out_frames);
            r.push(&ramp(need));
            let mut out = vec![0.0; out_frames * 2];
            assert!(r.pull(&mut out), "pull must succeed after pushing needed_input");
            produced += out_frames;
        }
        assert_eq!(produced, 480 * 50);
        assert!(r.fifo.len() <= 8, "fifo must not grow unboundedly");
    }

    #[test]
    fn downsampling_48k_to_44_1k_many_blocks_has_no_drift_or_growth() {
        let mut r = LinearResampler::new(48_000, 44_100);
        let out_frames = 441;
        let mut produced = 0;
        for _ in 0..50 {
            let need = r.needed_input(out_frames);
            r.push(&ramp(need));
            let mut out = vec![0.0; out_frames * 2];
            assert!(r.pull(&mut out), "pull must succeed after pushing needed_input");
            produced += out_frames;
        }
        assert_eq!(produced, 441 * 50);
        assert!(r.fifo.len() <= 8, "fifo must not grow unboundedly");
    }

    #[test]
    fn available_out_matches_what_pull_accepts() {
        let mut r = LinearResampler::new(48_000, 44_100);
        r.push(&ramp(100));
        let n = r.available_out();
        let mut out = vec![0.0; n * 2];
        assert!(r.pull(&mut out));
        let mut one_more = vec![0.0; 2];
        assert!(!r.pull(&mut one_more));
    }
}
