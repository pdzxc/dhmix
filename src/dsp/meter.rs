//! Peak meter with ballistic fall-off, published to the UI through atomics.

use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Default)]
pub struct AtomicPeak {
    bits: [AtomicU32; 2],
}

impl AtomicPeak {
    pub fn store(&self, left: f32, right: f32) {
        self.bits[0].store(left.to_bits(), Ordering::Relaxed);
        self.bits[1].store(right.to_bits(), Ordering::Relaxed);
    }

    pub fn load(&self) -> [f32; 2] {
        [
            f32::from_bits(self.bits[0].load(Ordering::Relaxed)),
            f32::from_bits(self.bits[1].load(Ordering::Relaxed)),
        ]
    }
}

pub fn block_peak(buf: &[f32]) -> [f32; 2] {
    let mut peak = [0.0f32; 2];
    for frame in buf.chunks_exact(2) {
        peak[0] = peak[0].max(frame[0].abs());
        peak[1] = peak[1].max(frame[1].abs());
    }
    peak
}

/// Smooths block peaks so the UI meter falls gradually instead of flickering.
#[derive(Clone, Debug, Default)]
pub struct MeterBallistics {
    level: [f32; 2],
}

impl MeterBallistics {
    pub fn update(&mut self, peak: [f32; 2]) -> [f32; 2] {
        for (level, peak) in self.level.iter_mut().zip(peak) {
            *level = if peak > *level { peak } else { *level * 0.85 };
        }
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_peak_reports_per_channel_maximum() {
        assert_eq!(block_peak(&[0.1, -0.9, 0.5, 0.2]), [0.5, 0.9]);
    }

    #[test]
    fn meter_falls_gradually() {
        let mut m = MeterBallistics::default();
        assert_eq!(m.update([1.0, 1.0]), [1.0, 1.0]);
        let next = m.update([0.0, 0.0]);
        assert!(next[0] > 0.5 && next[0] < 1.0);
    }
}
