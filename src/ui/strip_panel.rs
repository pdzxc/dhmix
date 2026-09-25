//! One input strip: routing LEDs, effect knobs and LEDs, mono/solo/mute on the left; ruler fader
//! and segmented meter on the right; detailed effect sliders below.

use super::widgets::{self, COLOR_ACTIVE, COLOR_MUTE, COLOR_SOLO, COLOR_VIRTUAL, LED_PAIR, LED_ROUTE, LED_TRIPLE};
use crate::dsp::compressor::CompressorSettings;
use crate::dsp::gate::GateSettings;
use crate::engine::{Meters, StripSettings};
use crate::{bus_name, NUM_BUSES, NUM_HW_BUSES};
use egui::Ui;

const TIP_HARDWARE_ROUTE: &str = "Hardware out: speakers or headphones";
const TIP_VIRTUAL_ROUTE: &str = "Virtual out: what OBS, Discord or a call hears";
/// Knob travel, like the console's 0..10 dials.
const KNOB_RANGE: std::ops::RangeInclusive<f32> = 0.0..=10.0;
const KNOB_OFF: f32 = 0.05;
/// The dials span the same ranges as the fine-tune sliders, so neither control can snap the other.
const COMP_DB_PER_STEP: f32 = 6.0;
const GATE_FLOOR_DB: f32 = -80.0;
const GATE_DB_PER_STEP: f32 = 8.0;

/// Maps the compressor to one 0..10 dial: 0 is off, 10 is a -60 dB threshold.
pub fn comp_amount(c: &CompressorSettings) -> f32 {
    if c.enabled {
        (-c.threshold_db / COMP_DB_PER_STEP).clamp(0.0, 10.0)
    } else {
        0.0
    }
}

pub fn set_comp_amount(c: &mut CompressorSettings, amount: f32) {
    c.enabled = amount > KNOB_OFF;
    if c.enabled {
        c.threshold_db = -amount * COMP_DB_PER_STEP;
    }
}

/// Maps the gate to one 0..10 dial: 0 is off, 10 opens only above 0 dB.
pub fn gate_amount(g: &GateSettings) -> f32 {
    if g.enabled {
        ((g.threshold_db - GATE_FLOOR_DB) / GATE_DB_PER_STEP).clamp(0.0, 10.0)
    } else {
        0.0
    }
}

pub fn set_gate_amount(g: &mut GateSettings, amount: f32) {
    g.enabled = amount > KNOB_OFF;
    if g.enabled {
        g.threshold_db = GATE_FLOOR_DB + amount * GATE_DB_PER_STEP;
    }
}

pub struct StripView<'a> {
    pub index: usize,
    pub settings: &'a mut StripSettings,
    pub meters: &'a Meters,
    pub any_solo: bool,
}

impl StripView<'_> {
    pub fn show(&mut self, ui: &mut Ui) {
        let silenced = self.settings.mute || (self.any_solo && !self.settings.solo);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(widgets::STRIP_LEFT_WIDTH);
                self.left_column(ui);
            });
            ui.scope(|ui| {
                if silenced {
                    ui.set_opacity(0.45);
                }
                widgets::fader(ui, &mut self.settings.gain_db);
                widgets::meter(ui, self.meters.strips[self.index].load());
            });
        });
        if self.any_solo && !self.settings.solo && !self.settings.mute {
            widgets::hint(ui, "Silent while another strip is soloed");
        }
        ui.spacing_mut().slider_width = widgets::DETAIL_SLIDER_WIDTH;
        ui.add(egui::Slider::new(&mut self.settings.pan, -1.0..=1.0).show_value(false).text("pan"))
            .on_hover_text("Left / right balance. Double-click to centre.");
        self.fx_details(ui);
    }

    fn left_column(&mut self, ui: &mut Ui) {
        widgets::section(ui, "Send to");
        routing_rows(ui, &mut self.settings.routing, self.index);
        widgets::section(ui, "Effects");
        let s = &mut self.settings;
        ui.horizontal(|ui| {
            let mut comp = comp_amount(&s.comp);
            if widgets::knob(ui, &mut comp, KNOB_RANGE, 0.0, "COMP", "Even out loud and quiet moments.") {
                set_comp_amount(&mut s.comp, comp);
            }
            let mut gate = gate_amount(&s.gate);
            if widgets::knob(ui, &mut gate, KNOB_RANGE, 0.0, "GATE", "Close the mic when you are not talking.") {
                set_gate_amount(&mut s.gate, gate);
            }
        });
        ui.horizontal(|ui| {
            widgets::led(ui, &mut s.denoise, "DENOISE", COLOR_ACTIVE, LED_PAIR, "Remove fans, hum and keyboard noise (RNNoise).");
            widgets::led(ui, &mut s.eq.enabled, "EQ", COLOR_ACTIVE, LED_PAIR, "Shape the tone: bass, mid, treble, low cut.");
        });
        ui.horizontal(|ui| {
            widgets::led(ui, &mut s.echo.enabled, "ECHO", COLOR_ACTIVE, LED_PAIR, "Repeating delay effect.");
            widgets::led(ui, &mut s.reverb.enabled, "REVERB", COLOR_ACTIVE, LED_PAIR, "Room ambience.");
        });
        ui.add_space(widgets::SECTION_GAP);
        ui.horizontal(|ui| {
            widgets::led(ui, &mut s.mono, "MONO", COLOR_ACTIVE, LED_TRIPLE, "Sum left and right into the centre. Use for a single microphone.");
            widgets::led(ui, &mut s.solo, "SOLO", COLOR_SOLO, LED_TRIPLE, "Hear only soloed strips.");
            widgets::led(ui, &mut s.mute, "MUTE", COLOR_MUTE, LED_TRIPLE, "Silence this strip on every bus.");
        });
    }

    fn fx_details(&mut self, ui: &mut Ui) {
        let s = &mut self.settings;
        let index = self.index;
        let meters = self.meters;
        egui::CollapsingHeader::new("Fine-tune effects").id_salt(("fx", index)).show(ui, |ui| {
            ui.spacing_mut().slider_width = widgets::DETAIL_SLIDER_WIDTH;
            widgets::section(ui, "Gate");
            if !s.gate.enabled {
                widgets::hint(ui, "Off. Turn the GATE knob to enable.");
            }
            ui.add(egui::Slider::new(&mut s.gate.threshold_db, -80.0..=0.0).text("open at dB"));
            ui.add(egui::Slider::new(&mut s.gate.release_ms, 10.0..=1000.0).text("release ms"));
            let open = meters.gate_open[index].load(std::sync::atomic::Ordering::Relaxed);
            widgets::hint(ui, if s.gate.enabled && !open { "Gate closed" } else { "Gate open" });

            widgets::section(ui, "EQ");
            widgets::led(ui, &mut s.eq.low_cut, "LOW CUT 80 Hz", COLOR_ACTIVE, LED_PAIR, "Remove rumble and desk thumps.");
            ui.add(egui::Slider::new(&mut s.eq.bass_db, -12.0..=12.0).text("bass dB"));
            ui.add(egui::Slider::new(&mut s.eq.mid_db, -12.0..=12.0).text("mid dB"));
            ui.add(egui::Slider::new(&mut s.eq.treble_db, -12.0..=12.0).text("treble dB"));

            widgets::section(ui, "Compressor");
            if !s.comp.enabled {
                widgets::hint(ui, "Off. Turn the COMP knob to enable.");
            }
            ui.add(egui::Slider::new(&mut s.comp.threshold_db, -60.0..=0.0).text("threshold dB"));
            ui.add(egui::Slider::new(&mut s.comp.ratio, 1.0..=20.0).text("ratio"));
            ui.add(egui::Slider::new(&mut s.comp.attack_ms, 0.1..=100.0).logarithmic(true).text("attack ms"));
            ui.add(egui::Slider::new(&mut s.comp.release_ms, 10.0..=1000.0).logarithmic(true).text("release ms"));
            ui.add(egui::Slider::new(&mut s.comp.makeup_db, 0.0..=24.0).text("makeup dB"));
            widgets::hint(ui, format!("Reducing {:.1} dB", meters.reduction(index)));

            widgets::section(ui, "Echo");
            ui.add(egui::Slider::new(&mut s.echo.time_ms, 20.0..=2000.0).text("time ms"));
            ui.add(egui::Slider::new(&mut s.echo.feedback, 0.0..=0.95).text("feedback"));
            ui.add(egui::Slider::new(&mut s.echo.mix, 0.0..=1.0).text("mix"));

            widgets::section(ui, "Reverb");
            ui.add(egui::Slider::new(&mut s.reverb.size, 0.0..=1.0).text("size"));
            ui.add(egui::Slider::new(&mut s.reverb.damping, 0.0..=1.0).text("damping"));
            ui.add(egui::Slider::new(&mut s.reverb.mix, 0.0..=1.0).text("mix"));
        });
    }
}

/// Two rows of LEDs: hardware outs A1..A5 in green, then virtual outs B1..B2 in blue.
fn routing_rows(ui: &mut Ui, routing: &mut [bool; NUM_BUSES], strip: usize) {
    let (hardware, virtual_buses) = routing.split_at_mut(NUM_HW_BUSES);
    ui.horizontal(|ui| {
        for (b, on) in hardware.iter_mut().enumerate() {
            ui.push_id((strip, b), |ui| widgets::led(ui, on, &bus_name(b), COLOR_ACTIVE, LED_ROUTE, TIP_HARDWARE_ROUTE));
        }
    });
    ui.horizontal(|ui| {
        for (offset, on) in virtual_buses.iter_mut().enumerate() {
            let b = NUM_HW_BUSES + offset;
            ui.push_id((strip, b), |ui| widgets::led(ui, on, &bus_name(b), COLOR_VIRTUAL, LED_ROUTE, TIP_VIRTUAL_ROUTE));
        }
        widgets::hint(ui, "stream / call");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comp_knob_round_trips_and_zero_turns_it_off() {
        let mut c = CompressorSettings::default();
        set_comp_amount(&mut c, 5.0);
        assert!(c.enabled);
        assert_eq!(c.threshold_db, -30.0);
        assert!((comp_amount(&c) - 5.0).abs() < 1e-6);
        set_comp_amount(&mut c, 0.0);
        assert!(!c.enabled);
        assert_eq!(comp_amount(&c), 0.0);
    }

    #[test]
    fn gate_knob_round_trips_within_the_dial_range() {
        let mut g = GateSettings::default();
        set_gate_amount(&mut g, 10.0);
        assert!(g.enabled);
        assert_eq!(g.threshold_db, 0.0);
        set_gate_amount(&mut g, 2.5);
        assert_eq!(g.threshold_db, -60.0);
        assert!((gate_amount(&g) - 2.5).abs() < 1e-6);
    }

    #[test]
    fn disabled_effects_read_as_zero_on_the_dial() {
        let c = CompressorSettings { enabled: false, threshold_db: -30.0, ..Default::default() };
        assert_eq!(comp_amount(&c), 0.0);
        let g = GateSettings { enabled: false, threshold_db: -30.0, ..Default::default() };
        assert_eq!(gate_amount(&g), 0.0);
    }

    #[test]
    fn set_comp_amount_at_exactly_the_off_threshold_stays_off() {
        let mut c = CompressorSettings::default();
        set_comp_amount(&mut c, KNOB_OFF);
        assert!(!c.enabled, "amount == KNOB_OFF should not enable the compressor (condition is `>`, not `>=`)");
        assert_eq!(comp_amount(&c), 0.0);
    }

    #[test]
    fn comp_amount_clamps_values_above_ten_on_read_back() {
        // Simulates a preset or hand-edited settings with a threshold beyond the dial's range.
        let c = CompressorSettings { enabled: true, threshold_db: -100.0, ..Default::default() };
        assert_eq!(comp_amount(&c), 10.0);
    }

    #[test]
    fn gate_amount_clamps_to_ten_when_threshold_is_above_minus_20_db() {
        let g = GateSettings { enabled: true, threshold_db: 0.0, ..Default::default() };
        assert_eq!(gate_amount(&g), 10.0);
    }
}

#[cfg(test)]
mod range_tests {
    use super::*;

    #[test]
    fn dials_cover_the_full_slider_ranges_so_nothing_snaps() {
        let mut c = CompressorSettings { enabled: true, threshold_db: -60.0, ..Default::default() };
        let amount = comp_amount(&c);
        set_comp_amount(&mut c, amount);
        assert_eq!(c.threshold_db, -60.0);
        let mut g = GateSettings { enabled: true, threshold_db: -80.0, ..Default::default() };
        let amount = gate_amount(&g);
        set_gate_amount(&mut g, amount);
        assert!(!g.enabled, "the slider floor is the dial's off position");
        g = GateSettings { enabled: true, threshold_db: -1.0, ..Default::default() };
        let amount = gate_amount(&g);
        set_gate_amount(&mut g, amount);
        assert!((g.threshold_db + 1.0).abs() < 1e-4);
    }
}
