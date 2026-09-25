//! Signal-processing blocks. Every block processes interleaved stereo `f32` in place and
//! carries its own state, so the engine can own one instance per strip or bus.

pub mod biquad;
pub mod compressor;
pub mod denoise;
pub mod echo;
pub mod eq;
pub mod gate;
pub mod limiter;
pub mod meter;
pub mod reverb;

/// Converts a time constant in milliseconds to a one-pole smoothing coefficient.
pub fn time_coeff(ms: f32, sample_rate: f32) -> f32 {
    if ms <= 0.0 {
        0.0
    } else {
        (-1.0 / (ms * 0.001 * sample_rate)).exp()
    }
}
