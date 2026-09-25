//! DHMIX: a free Voicemeeter-style mixer for streaming.
//!
//! Layout:
//! - `dsp`    – pure signal-processing blocks (EQ, compressor, gate, echo, reverb, denoiser, limiter).
//! - `engine` – the mixer graph: strips → routing matrix → buses, plus the shared settings/meters.
//! - `audio`  – device enumeration and cpal streams (WASAPI on Windows), ring buffers, resampling.
//! - `ui`     – the egui mixer window.
//! - `apps`   – running applications with audio sessions and per-app device routing (Windows).

// DSP loops iterate interleaved stereo frames with `chunks_exact(2)`; that reads better than
// `as_chunks::<2>()` and compiles to the same code.
#![allow(clippy::chunks_exact_to_as_chunks)]

pub mod app_settings;
pub mod apps;
pub mod audio;
pub mod dsp;
pub mod engine;
pub mod hotkeys;
pub mod preset;
pub mod recorder;
pub mod startup;
pub mod ui;

/// The "DH" badge, rendered by assets/make_icon.swift as straight RGBA so no image decoder is
/// needed. It is the window, Dock, taskbar and tray icon.
pub const APP_ICON_RGBA: &[u8] = include_bytes!("../assets/icon-256.rgba");
pub const APP_ICON_SIZE: u32 = 256;

/// Engine sample rate. Fixed at 48 kHz because RNNoise (the denoiser) only works at 48 kHz.
pub const SAMPLE_RATE: u32 = 48_000;
/// Frames per engine block: 10 ms, which is exactly one RNNoise frame.
pub const BLOCK_FRAMES: usize = 480;
/// Engine channel count. Every strip and bus is stereo.
pub const CHANNELS: usize = 2;

pub const NUM_HW_STRIPS: usize = 3;
pub const NUM_VIRT_STRIPS: usize = 2;
/// The soundboard / music player strip. It has no input device: its audio comes from files.
pub const NUM_PLAYER_STRIPS: usize = 1;
pub const NUM_STRIPS: usize = NUM_HW_STRIPS + NUM_VIRT_STRIPS + NUM_PLAYER_STRIPS;
pub const PLAYER_STRIP: usize = NUM_HW_STRIPS + NUM_VIRT_STRIPS;

pub const NUM_HW_BUSES: usize = 3;
pub const NUM_VIRT_BUSES: usize = 2;
pub const NUM_BUSES: usize = NUM_HW_BUSES + NUM_VIRT_BUSES;

pub fn strip_name(i: usize) -> String {
    if i < NUM_HW_STRIPS {
        format!("HW {}", i + 1)
    } else if i < PLAYER_STRIP {
        format!("VIRT {}", i - NUM_HW_STRIPS + 1)
    } else {
        "PLAYER".to_string()
    }
}

pub fn bus_name(i: usize) -> String {
    if i < NUM_HW_BUSES {
        format!("A{}", i + 1)
    } else {
        format!("B{}", i - NUM_HW_BUSES + 1)
    }
}

pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

pub fn gain_to_db(gain: f32) -> f32 {
    if gain <= 1e-9 {
        -180.0
    } else {
        20.0 * gain.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_asset_is_exactly_the_declared_square_of_straight_rgba() {
        assert_eq!(APP_ICON_RGBA.len(), (APP_ICON_SIZE * APP_ICON_SIZE * 4) as usize);
    }

    /// The bundled LICENSE file and Cargo.toml's declared license must agree: both MIT, so a
    /// packager reading either one sees the same terms.
    #[test]
    fn license_file_and_cargo_manifest_both_say_mit() {
        const LICENSE_TEXT: &str = include_str!("../LICENSE");
        assert!(LICENSE_TEXT.contains("MIT License"), "LICENSE file should contain \"MIT License\"");
        assert_eq!(env!("CARGO_PKG_LICENSE"), "MIT", "Cargo.toml's license field should be \"MIT\"");
    }
}
