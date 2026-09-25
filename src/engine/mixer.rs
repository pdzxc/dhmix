//! Strips → routing matrix → buses, on whole blocks of interleaved stereo.

use super::settings::{BusSettings, Meters, MixSettings, StripSettings};
use crate::dsp::biquad::{Coeffs, StereoBiquad};
use crate::dsp::compressor::Compressor;
use crate::dsp::denoise::Denoiser;
use crate::dsp::echo::Echo;
use crate::dsp::eq::{Eq, BASS_HZ, TREBLE_HZ};
use crate::dsp::gate::Gate;
use crate::dsp::limiter::Limiter;
use crate::dsp::meter::{block_peak, MeterBallistics};
use crate::dsp::reverb::Reverb;
use crate::{db_to_gain, BLOCK_FRAMES, CHANNELS, NUM_BUSES, NUM_STRIPS};
use std::f32::consts::FRAC_PI_4;
use std::sync::atomic::Ordering;

struct StripDsp {
    denoiser: Denoiser,
    gate: Gate,
    eq: Eq,
    comp: Compressor,
    echo: Echo,
    reverb: Reverb,
    meter: MeterBallistics,
    out: Vec<f32>,
}

impl StripDsp {
    fn new(sample_rate: f32) -> Self {
        Self {
            denoiser: Denoiser::new(),
            gate: Gate::new(sample_rate),
            eq: Eq::new(sample_rate),
            comp: Compressor::new(sample_rate),
            echo: Echo::new(sample_rate),
            reverb: Reverb::new(sample_rate),
            meter: MeterBallistics::default(),
            out: vec![0.0; BLOCK_FRAMES * CHANNELS],
        }
    }

    fn apply(&mut self, s: &StripSettings) {
        self.denoiser.enabled = s.denoise;
        self.gate.apply(s.gate);
        self.eq.apply(s.eq);
        self.comp.apply(s.comp);
        self.echo.apply(s.echo);
        self.reverb.apply(s.reverb);
    }

    fn process(&mut self, input: &[f32], s: &StripSettings, audible: bool) {
        self.out.copy_from_slice(input);
        let buf = &mut self.out[..];
        self.denoiser.process(buf);
        self.gate.process(buf);
        self.eq.process(buf);
        self.comp.process(buf);
        self.echo.process(buf);
        self.reverb.process(buf);
        apply_gain_mono_pan(buf, s, audible);
    }
}

fn apply_gain_mono_pan(buf: &mut [f32], s: &StripSettings, audible: bool) {
    if !audible {
        buf.fill(0.0);
        return;
    }
    let gain = db_to_gain(s.gain_db);
    // Constant-power pan law.
    let angle = (s.pan.clamp(-1.0, 1.0) + 1.0) * FRAC_PI_4;
    let (pan_r, pan_l) = angle.sin_cos();
    for frame in buf.chunks_exact_mut(2) {
        if s.mono {
            let m = (frame[0] + frame[1]) * 0.5;
            frame[0] = m;
            frame[1] = m;
        }
        frame[0] *= gain * pan_l * std::f32::consts::SQRT_2;
        frame[1] *= gain * pan_r * std::f32::consts::SQRT_2;
    }
}

struct BusDsp {
    settings: BusSettings,
    bass: StereoBiquad,
    treble: StereoBiquad,
    limiter: Limiter,
    meter: MeterBallistics,
}

impl BusDsp {
    fn new(sample_rate: f32) -> Self {
        Self {
            settings: BusSettings::default(),
            bass: StereoBiquad::default(),
            treble: StereoBiquad::default(),
            limiter: Limiter::new(sample_rate, -0.5),
            meter: MeterBallistics::default(),
        }
    }

    fn apply(&mut self, s: &BusSettings, sample_rate: f32) {
        if s.bass_db != self.settings.bass_db {
            self.bass.set_coeffs(Coeffs::low_shelf(sample_rate, BASS_HZ, s.bass_db, 0.9));
        }
        if s.treble_db != self.settings.treble_db {
            self.treble.set_coeffs(Coeffs::high_shelf(sample_rate, TREBLE_HZ, s.treble_db, 0.9));
        }
        self.limiter.enabled = s.limiter;
        self.settings = *s;
    }

    fn process(&mut self, buf: &mut [f32]) {
        let s = self.settings;
        if s.mute {
            buf.fill(0.0);
            return;
        }
        if s.bass_db != 0.0 {
            self.bass.process(buf);
        }
        if s.treble_db != 0.0 {
            self.treble.process(buf);
        }
        let gain = db_to_gain(s.gain_db);
        buf.iter_mut().for_each(|v| *v *= gain);
        self.limiter.process(buf);
    }
}

pub struct Mixer {
    sample_rate: f32,
    settings: MixSettings,
    strips: Vec<StripDsp>,
    buses: Vec<BusDsp>,
}

impl Mixer {
    pub fn new(sample_rate: f32) -> Self {
        let mut mixer = Self {
            sample_rate,
            settings: MixSettings::default(),
            strips: (0..NUM_STRIPS).map(|_| StripDsp::new(sample_rate)).collect(),
            buses: (0..NUM_BUSES).map(|_| BusDsp::new(sample_rate)).collect(),
        };
        mixer.set_settings(MixSettings::default());
        mixer
    }

    pub fn set_settings(&mut self, settings: MixSettings) {
        self.settings = settings;
        for (dsp, s) in self.strips.iter_mut().zip(settings.strips.iter()) {
            dsp.apply(s);
        }
        for (dsp, s) in self.buses.iter_mut().zip(settings.buses.iter()) {
            dsp.apply(s, self.sample_rate);
        }
    }

    /// Reads the current gain-reduction of a strip's compressor, in dB.
    pub fn strip_reduction_db(&self, strip: usize) -> f32 {
        self.strips[strip].comp.reduction_db
    }

    /// One block: `inputs[i]` is strip i's raw stereo audio, `outputs[b]` receives bus b's mix.
    /// Every slice must be exactly `BLOCK_FRAMES * CHANNELS` long.
    pub fn process(&mut self, inputs: &[Vec<f32>], outputs: &mut [Vec<f32>], meters: &Meters) {
        debug_assert_eq!(inputs.len(), NUM_STRIPS);
        debug_assert_eq!(outputs.len(), NUM_BUSES);
        let any_solo = self.settings.any_solo();

        for (i, dsp) in self.strips.iter_mut().enumerate() {
            let s = &self.settings.strips[i];
            let audible = !s.mute && (!any_solo || s.solo);
            dsp.process(&inputs[i], s, audible);
            let level = dsp.meter.update(block_peak(&dsp.out));
            meters.strips[i].store(level[0], level[1]);
            meters.set_reduction(i, dsp.comp.reduction_db);
            meters.gate_open[i].store(dsp.gate.open, Ordering::Relaxed);
        }

        for (b, out) in outputs.iter_mut().enumerate() {
            out.fill(0.0);
            for (i, dsp) in self.strips.iter().enumerate() {
                if self.settings.strips[i].routing[b] {
                    for (o, v) in out.iter_mut().zip(dsp.out.iter()) {
                        *o += v;
                    }
                }
            }
            self.buses[b].process(out);
            let level = self.buses[b].meter.update(block_peak(out));
            meters.buses[b].store(level[0], level[1]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NUM_HW_BUSES, SAMPLE_RATE};

    fn block(value: f32) -> Vec<f32> {
        vec![value; BLOCK_FRAMES * CHANNELS]
    }

    fn run(settings: MixSettings, inputs: Vec<Vec<f32>>) -> Vec<Vec<f32>> {
        let mut mixer = Mixer::new(SAMPLE_RATE as f32);
        mixer.set_settings(settings);
        let mut outputs = vec![block(0.0); NUM_BUSES];
        let meters = Meters::default();
        mixer.process(&inputs, &mut outputs, &meters);
        outputs
    }

    fn quiet_settings() -> MixSettings {
        let mut s = MixSettings::default();
        for strip in s.strips.iter_mut() {
            strip.routing = [false; NUM_BUSES];
        }
        s
    }

    #[test]
    fn strip_reaches_only_the_buses_it_is_routed_to() {
        let mut s = quiet_settings();
        s.strips[0].routing[0] = true;
        s.strips[0].routing[NUM_HW_BUSES] = true;
        let mut inputs = vec![block(0.0); NUM_STRIPS];
        inputs[0] = block(0.25);
        let out = run(s, inputs);
        assert!((out[0][0] - 0.25).abs() < 1e-6, "A1 gets it");
        assert!((out[NUM_HW_BUSES][0] - 0.25).abs() < 1e-6, "B1 gets it");
        assert_eq!(out[1][0], 0.0, "A2 stays silent");
    }

    #[test]
    fn two_strips_on_one_bus_are_summed() {
        let mut s = quiet_settings();
        s.strips[0].routing[0] = true;
        s.strips[1].routing[0] = true;
        let mut inputs = vec![block(0.0); NUM_STRIPS];
        inputs[0] = block(0.2);
        inputs[1] = block(0.3);
        let out = run(s, inputs);
        assert!((out[0][0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn mute_silences_and_solo_isolates() {
        let mut s = quiet_settings();
        s.strips[0].routing[0] = true;
        s.strips[1].routing[0] = true;
        s.strips[1].solo = true;
        let mut inputs = vec![block(0.0); NUM_STRIPS];
        inputs[0] = block(0.2);
        inputs[1] = block(0.3);
        let out = run(s, inputs.clone());
        assert!((out[0][0] - 0.3).abs() < 1e-6, "only the soloed strip is heard");

        s.strips[1].solo = false;
        s.strips[1].mute = true;
        let out = run(s, inputs);
        assert!((out[0][0] - 0.2).abs() < 1e-6, "muted strip drops out");
    }

    #[test]
    fn gain_and_mono_are_applied() {
        let mut s = quiet_settings();
        s.strips[0].routing[0] = true;
        s.strips[0].gain_db = -6.0;
        s.strips[0].mono = true;
        let mut inputs = vec![block(0.0); NUM_STRIPS];
        let mut left_only = block(0.0);
        for frame in left_only.chunks_exact_mut(2) {
            frame[0] = 0.5;
        }
        inputs[0] = left_only;
        let out = run(s, inputs);
        let expected = 0.25 * db_to_gain(-6.0);
        assert!((out[0][0] - expected).abs() < 1e-4, "left {}", out[0][0]);
        assert!((out[0][1] - expected).abs() < 1e-4, "right {}", out[0][1]);
    }

    #[test]
    fn bus_mute_and_gain_work() {
        let mut s = quiet_settings();
        s.strips[0].routing[0] = true;
        s.strips[0].routing[1] = true;
        s.buses[0].mute = true;
        s.buses[1].gain_db = -20.0;
        let mut inputs = vec![block(0.0); NUM_STRIPS];
        inputs[0] = block(0.5);
        let out = run(s, inputs);
        assert_eq!(out[0][0], 0.0);
        assert!((out[1][0] - 0.05).abs() < 1e-4);
    }

    #[test]
    fn pan_law_keeps_centre_gain() {
        let mut buf = vec![0.3, -0.2, 0.3, -0.2];
        let s = StripSettings { pan: 0.0, gain_db: 0.0, ..StripSettings::default() };
        apply_gain_mono_pan(&mut buf, &s, true);
        assert!((buf[0] - 0.3).abs() < 1e-6, "left unchanged at centre pan, {}", buf[0]);
        assert!((buf[1] - (-0.2)).abs() < 1e-6, "right unchanged at centre pan, {}", buf[1]);
    }

    #[test]
    fn player_strip_solo_and_mute_together_stays_silent() {
        let mut s = quiet_settings();
        s.strips[crate::PLAYER_STRIP].routing[NUM_HW_BUSES] = true;
        s.strips[crate::PLAYER_STRIP].solo = true;
        s.strips[crate::PLAYER_STRIP].mute = true;
        let mut inputs = vec![block(0.0); NUM_STRIPS];
        inputs[crate::PLAYER_STRIP] = block(0.4);
        let out = run(s, inputs);
        assert_eq!(out[NUM_HW_BUSES][0], 0.0, "mute wins even though the strip is also soloed");
    }

    #[test]
    fn default_settings_send_everything_to_a1_and_mic_and_player_to_b1() {
        let s = MixSettings::default();
        assert!(s.strips.iter().all(|st| st.routing[0]));
        assert!(s.strips[0].routing[NUM_HW_BUSES]);
        assert!(s.strips[crate::PLAYER_STRIP].routing[NUM_HW_BUSES]);
        assert!(!s.strips[1].routing[NUM_HW_BUSES]);
    }
}
