//! The mixer graph. `Mixer` is pure and testable: strips in, buses out. `runner` wraps it in a
//! thread fed by the audio ring buffers.

pub mod mixer;
pub mod player;
pub mod runner;
pub mod settings;

pub use mixer::Mixer;
pub use settings::{BusSettings, Meters, MixSettings, StripSettings};
