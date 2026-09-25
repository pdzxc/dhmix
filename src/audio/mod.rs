//! Device I/O. `devices` lists endpoints, `resample` bridges device rates to the engine's 48 kHz,
//! `streams` owns the cpal streams and the ring buffers the engine reads from and writes to.

pub mod devices;
pub mod resample;
pub mod streams;

pub use devices::{list_devices, DeviceList};
pub use streams::{AudioIo, InputSlot, IoSettings, OutputSlot};
