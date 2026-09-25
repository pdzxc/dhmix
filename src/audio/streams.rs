//! cpal streams wired to SPSC ring buffers. Input callbacks convert the device's channel count and
//! rate to engine stereo/48 kHz and push into the strip's ring; output callbacks pull the bus's
//! ring and convert back to the device format.

use super::devices::{find_input, find_output};
use super::resample::LinearResampler;
use crate::{BLOCK_FRAMES, CHANNELS, NUM_BUSES, NUM_STRIPS, SAMPLE_RATE};
use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleRate, Stream, StreamConfig};
use parking_lot::Mutex;
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Ring capacity in engine blocks: bounds the added latency.
pub const RING_BLOCKS: usize = 6;

pub type InputSlot = Arc<Mutex<Option<HeapCons<f32>>>>;
pub type OutputSlot = Arc<Mutex<Option<HeapProd<f32>>>>;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IoSettings {
    pub strip_inputs: Vec<Option<String>>,
    pub bus_outputs: Vec<Option<String>>,
}

impl IoSettings {
    pub fn empty() -> Self {
        Self { strip_inputs: vec![None; NUM_STRIPS], bus_outputs: vec![None; NUM_BUSES] }
    }
}

/// Owns the live streams. Not `Send` on every platform, so it lives on the UI thread.
pub struct AudioIo {
    input_slots: Vec<InputSlot>,
    output_slots: Vec<OutputSlot>,
    input_streams: Vec<Option<Stream>>,
    output_streams: Vec<Option<Stream>>,
    pub settings: IoSettings,
    pub errors: Vec<String>,
}

impl AudioIo {
    pub fn new() -> Self {
        Self {
            input_slots: (0..NUM_STRIPS).map(|_| Arc::new(Mutex::new(None))).collect(),
            output_slots: (0..NUM_BUSES).map(|_| Arc::new(Mutex::new(None))).collect(),
            input_streams: (0..NUM_STRIPS).map(|_| None).collect(),
            output_streams: (0..NUM_BUSES).map(|_| None).collect(),
            settings: IoSettings::empty(),
            errors: Vec::new(),
        }
    }

    pub fn input_slots(&self) -> Vec<InputSlot> {
        self.input_slots.clone()
    }

    pub fn output_slots(&self) -> Vec<OutputSlot> {
        self.output_slots.clone()
    }

    pub fn apply(&mut self, settings: &IoSettings) {
        for i in 0..NUM_STRIPS {
            let wanted = settings.strip_inputs.get(i).cloned().flatten();
            if self.settings.strip_inputs.get(i).cloned().flatten() != wanted || self.input_streams[i].is_none() {
                self.set_strip_input(i, wanted);
            }
        }
        for b in 0..NUM_BUSES {
            let wanted = settings.bus_outputs.get(b).cloned().flatten();
            if self.settings.bus_outputs.get(b).cloned().flatten() != wanted || self.output_streams[b].is_none() {
                self.set_bus_output(b, wanted);
            }
        }
    }

    pub fn set_strip_input(&mut self, strip: usize, device_name: Option<String>) {
        self.input_streams[strip] = None;
        *self.input_slots[strip].lock() = None;
        self.settings.strip_inputs[strip] = device_name.clone();
        let Some(name) = device_name else { return };
        match open_input(&name, self.input_slots[strip].clone()) {
            Ok(stream) => self.input_streams[strip] = Some(stream),
            Err(e) => self.errors.push(format!("Strip {}: {e:#}", strip + 1)),
        }
    }

    pub fn set_bus_output(&mut self, bus: usize, device_name: Option<String>) {
        self.output_streams[bus] = None;
        *self.output_slots[bus].lock() = None;
        self.settings.bus_outputs[bus] = device_name.clone();
        let Some(name) = device_name else { return };
        match open_output(&name, self.output_slots[bus].clone()) {
            Ok(stream) => self.output_streams[bus] = Some(stream),
            Err(e) => self.errors.push(format!("Bus {}: {e:#}", crate::bus_name(bus))),
        }
    }
}

impl Default for AudioIo {
    fn default() -> Self {
        Self::new()
    }
}

fn ring() -> (HeapProd<f32>, HeapCons<f32>) {
    HeapRb::<f32>::new(BLOCK_FRAMES * CHANNELS * RING_BLOCKS).split()
}

/// Prefers the engine rate; otherwise takes the device default and resamples.
fn pick_config(default: cpal::SupportedStreamConfig, supported: Vec<cpal::SupportedStreamConfigRange>) -> StreamConfig {
    let wanted = SampleRate(SAMPLE_RATE);
    let at_engine_rate = supported
        .iter()
        .filter(|r| r.min_sample_rate() <= wanted && r.max_sample_rate() >= wanted && r.channels() >= 1)
        .max_by_key(|r| (r.channels() == 2, r.channels()));
    match at_engine_rate {
        Some(range) => StreamConfig { channels: range.channels(), sample_rate: wanted, buffer_size: cpal::BufferSize::Default },
        None => default.config(),
    }
}

fn open_input(name: &str, slot: InputSlot) -> Result<Stream> {
    let device = find_input(name).ok_or_else(|| anyhow!("input device '{name}' not found"))?;
    let default = device.default_input_config().context("no default input config")?;
    let supported: Vec<_> = device.supported_input_configs().map(|c| c.collect()).unwrap_or_default();
    let config = pick_config(default, supported);
    let channels = config.channels as usize;
    let (prod, cons) = ring();
    *slot.lock() = Some(cons);

    let mut prod = prod;
    let mut resampler = LinearResampler::new(config.sample_rate.0, SAMPLE_RATE);
    let mut stereo: Vec<f32> = Vec::with_capacity(BLOCK_FRAMES * CHANNELS * 4);
    let mut converted: Vec<f32> = Vec::with_capacity(BLOCK_FRAMES * CHANNELS * 4);
    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _| {
                to_stereo(data, channels, &mut stereo);
                if resampler.is_identity() {
                    prod.push_slice(&stereo);
                } else {
                    resampler.push(&stereo);
                    converted.resize(resampler.available_out() * CHANNELS, 0.0);
                    if resampler.pull(&mut converted) {
                        prod.push_slice(&converted);
                    }
                }
            },
            |e| log::warn!("input stream error: {e}"),
            None,
        )
        .context("build input stream")?;
    stream.play().context("start input stream")?;
    Ok(stream)
}

fn open_output(name: &str, slot: OutputSlot) -> Result<Stream> {
    let device = find_output(name).ok_or_else(|| anyhow!("output device '{name}' not found"))?;
    let default = device.default_output_config().context("no default output config")?;
    let supported: Vec<_> = device.supported_output_configs().map(|c| c.collect()).unwrap_or_default();
    let config = pick_config(default, supported);
    let channels = config.channels as usize;
    let (prod, cons) = ring();
    *slot.lock() = Some(prod);

    let mut cons = cons;
    let mut resampler = LinearResampler::new(SAMPLE_RATE, config.sample_rate.0);
    let mut stereo: Vec<f32> = Vec::with_capacity(BLOCK_FRAMES * CHANNELS * 4);
    let mut scratch: Vec<f32> = Vec::with_capacity(BLOCK_FRAMES * CHANNELS * 4);
    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _| {
                let frames = data.len() / channels;
                stereo.resize(frames * CHANNELS, 0.0);
                if resampler.is_identity() {
                    pop_or_silence(&mut cons, &mut stereo);
                } else {
                    scratch.resize(resampler.needed_input(frames) * CHANNELS, 0.0);
                    pop_or_silence(&mut cons, &mut scratch);
                    resampler.push(&scratch);
                    if !resampler.pull(&mut stereo) {
                        stereo.fill(0.0);
                    }
                }
                from_stereo(&stereo, channels, data);
            },
            |e| log::warn!("output stream error: {e}"),
            None,
        )
        .context("build output stream")?;
    stream.play().context("start output stream")?;
    Ok(stream)
}

fn pop_or_silence(cons: &mut HeapCons<f32>, out: &mut [f32]) {
    let got = cons.pop_slice(out);
    out[got..].fill(0.0);
}

/// Takes the first two channels of an interleaved buffer (mono is duplicated to both sides).
pub fn to_stereo(data: &[f32], channels: usize, out: &mut Vec<f32>) {
    out.clear();
    match channels {
        0 => {}
        1 => out.extend(data.iter().flat_map(|v| [*v, *v])),
        _ => out.extend(data.chunks_exact(channels).flat_map(|f| [f[0], f[1]])),
    }
}

/// Writes engine stereo into the device's interleaved layout, filling extra channels with silence.
pub fn from_stereo(stereo: &[f32], channels: usize, data: &mut [f32]) {
    match channels {
        0 => {}
        1 => {
            for (d, f) in data.iter_mut().zip(stereo.chunks_exact(2)) {
                *d = (f[0] + f[1]) * 0.5;
            }
        }
        _ => {
            for (d, f) in data.chunks_exact_mut(channels).zip(stereo.chunks_exact(2)) {
                d[0] = f[0];
                d[1] = f[1];
                d[2..].fill(0.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_input_is_duplicated_to_both_sides() {
        let mut out = Vec::new();
        to_stereo(&[0.1, 0.2], 1, &mut out);
        assert_eq!(out, vec![0.1, 0.1, 0.2, 0.2]);
    }

    #[test]
    fn multichannel_input_keeps_first_two_channels() {
        let mut out = Vec::new();
        to_stereo(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 4, &mut out);
        assert_eq!(out, vec![1.0, 2.0, 5.0, 6.0]);
    }

    #[test]
    fn stereo_to_multichannel_output_silences_extra_channels() {
        let mut data = vec![9.0; 6];
        from_stereo(&[0.5, 0.25, 0.1, 0.2], 3, &mut data);
        assert_eq!(data, vec![0.5, 0.25, 0.0, 0.1, 0.2, 0.0]);
    }

    #[test]
    fn stereo_to_mono_output_averages() {
        let mut data = vec![0.0; 2];
        from_stereo(&[0.5, 0.25, 1.0, 0.0], 1, &mut data);
        assert_eq!(data, vec![0.375, 0.5]);
    }
}
