//! The engine thread: pulls one block from every strip's ring, mixes, pushes into every bus's ring.
//! Paced by the first connected output device; free-running on a 10 ms timer when none is.

use super::player::Player;
use super::{Meters, MixSettings, Mixer};
use crate::audio::streams::RING_BLOCKS;
use crate::audio::{InputSlot, OutputSlot};
use crate::{BLOCK_FRAMES, CHANNELS, NUM_BUSES, NUM_STRIPS, PLAYER_STRIP, SAMPLE_RATE};
use crossbeam_channel::Sender;
use parking_lot::Mutex;
use ringbuf::traits::{Consumer, Observer, Producer};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const BLOCK_LEN: usize = BLOCK_FRAMES * CHANNELS;
/// Input rings are trimmed back to this many queued blocks to keep latency bounded.
const MAX_QUEUED_INPUT_BLOCKS: usize = 2;

/// Where a live recording goes. `tx` is set while recording; the engine copies the chosen bus.
#[derive(Default)]
pub struct RecordTap {
    pub tx: Mutex<Option<Sender<Vec<f32>>>>,
    pub bus: AtomicUsize,
}

pub struct EngineHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

pub struct EngineInputs {
    pub settings: Arc<Mutex<MixSettings>>,
    pub meters: Arc<Meters>,
    pub input_slots: Vec<InputSlot>,
    pub output_slots: Vec<OutputSlot>,
    pub player: Player,
    pub record: Arc<RecordTap>,
}

pub fn spawn(inputs: EngineInputs) -> EngineHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let thread = std::thread::Builder::new()
        .name("streammix-engine".into())
        .spawn(move || run(inputs, stop_flag))
        .expect("spawn engine thread");
    EngineHandle { stop, thread: Some(thread) }
}

fn run(mut e: EngineInputs, stop: Arc<AtomicBool>) {
    let mut mixer = Mixer::new(SAMPLE_RATE as f32);
    let mut in_bufs = vec![vec![0.0f32; BLOCK_LEN]; NUM_STRIPS];
    let mut out_bufs = vec![vec![0.0f32; BLOCK_LEN]; NUM_BUSES];
    let block_time = Duration::from_micros((BLOCK_FRAMES as u64 * 1_000_000) / SAMPLE_RATE as u64);
    let mut next_tick = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        wait_for_room(&e.output_slots, &mut next_tick, block_time);

        if let Some(settings) = e.settings.try_lock() {
            mixer.set_settings(*settings);
        }

        for (i, slot) in e.input_slots.iter().enumerate() {
            if i == PLAYER_STRIP {
                continue;
            }
            let connected = read_input(slot, &mut in_bufs[i], &e.meters);
            e.meters.input_connected[i].store(connected, Ordering::Relaxed);
        }
        e.player.render(&mut in_bufs[PLAYER_STRIP]);
        e.meters.input_connected[PLAYER_STRIP].store(true, Ordering::Relaxed);

        mixer.process(&in_bufs, &mut out_bufs, &e.meters);

        for (b, slot) in e.output_slots.iter().enumerate() {
            let connected = write_output(slot, &out_bufs[b], &e.meters);
            e.meters.output_connected[b].store(connected, Ordering::Relaxed);
        }
        tap_recording(&e.record, &out_bufs);
    }
}

/// Blocks until the clock output has room for a block, or a timer tick if nothing is connected.
/// The clock is the first connected output. If the UI thread holds that slot's lock at this
/// instant (it is swapping the device), the loop falls through to the timer for one block, which
/// is harmless: the ring buffers absorb a single early or late block.
fn wait_for_room(outputs: &[OutputSlot], next_tick: &mut Instant, block_time: Duration) {
    let deadline = Instant::now() + block_time * 4;
    loop {
        let mut clock_found = false;
        for slot in outputs {
            if let Some(guard) = slot.try_lock() {
                if let Some(prod) = guard.as_ref() {
                    clock_found = true;
                    if prod.vacant_len() >= BLOCK_LEN {
                        *next_tick = Instant::now();
                        return;
                    }
                    break;
                }
            }
        }
        if !clock_found {
            *next_tick += block_time;
            let now = Instant::now();
            if *next_tick > now {
                std::thread::sleep(*next_tick - now);
            } else if now - *next_tick > block_time * 4 {
                *next_tick = now;
            }
            return;
        }
        if Instant::now() >= deadline {
            return;
        }
        std::thread::sleep(Duration::from_micros(500));
    }
}

/// Fills `buf` from the strip's ring. Returns whether a device is connected.
fn read_input(slot: &InputSlot, buf: &mut [f32], meters: &Meters) -> bool {
    let Some(mut guard) = slot.try_lock() else {
        buf.fill(0.0);
        return true;
    };
    let Some(cons) = guard.as_mut() else {
        buf.fill(0.0);
        return false;
    };
    let queued_blocks = cons.occupied_len() / BLOCK_LEN;
    if queued_blocks > MAX_QUEUED_INPUT_BLOCKS {
        cons.skip((queued_blocks - MAX_QUEUED_INPUT_BLOCKS) * BLOCK_LEN);
    }
    let got = cons.pop_slice(buf);
    if got < buf.len() {
        buf[got..].fill(0.0);
        meters.underruns.fetch_add(1, Ordering::Relaxed);
    }
    true
}

/// Pushes a block into the bus's ring, dropping it when the device is behind. Returns connected.
fn write_output(slot: &OutputSlot, buf: &[f32], meters: &Meters) -> bool {
    let Some(mut guard) = slot.try_lock() else { return true };
    let Some(prod) = guard.as_mut() else { return false };
    if prod.vacant_len() >= buf.len() {
        prod.push_slice(buf);
    } else {
        meters.overruns.fetch_add(1, Ordering::Relaxed);
    }
    true
}

fn tap_recording(record: &RecordTap, out_bufs: &[Vec<f32>]) {
    if let Some(guard) = record.tx.try_lock() {
        if let Some(tx) = guard.as_ref() {
            let bus = record.bus.load(Ordering::Relaxed).min(NUM_BUSES - 1);
            let _ = tx.try_send(out_bufs[bus].clone());
        }
    }
}

/// Ring capacity as latency, for the settings display.
pub fn ring_latency_ms() -> f32 {
    RING_BLOCKS as f32 * BLOCK_FRAMES as f32 * 1000.0 / SAMPLE_RATE as f32
}
