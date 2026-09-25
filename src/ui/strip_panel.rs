//! One input strip: routing LEDs, mono/solo/mute and effect LEDs on the left, ruler fader and
//! meter on the right; below them the COMP / GATE knobs and the tone-and-echo pad. Detailed
//! effect sliders open in a floating window so the strip never grows.

use super::widgets::{self, Geometry, COLOR_ACTIVE, COLOR_MUTE, COLOR_SOLO, COLOR_VIRTUAL};
use crate::dsp::compressor::CompressorSettings;
use crate::dsp::echo::EchoSettings;
use crate::dsp::eq::EqSettings;
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
/// Full tilt on the pad is this much bass cut and treble boost (or the reverse).
const TILT_DB: f32 = 8.0;
/// Top of the pad is this much echo in the mix.
const ECHO_MAX_MIX: f32 = 0.6;
const PAD_OFF: f32 = 0.02;

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

/// Pad X: -1 (warm, bass up / treble down) .. +1 (bright). Reads 0 while the EQ is off.
pub fn tone_tilt(eq: &EqSettings) -> f32 {
    if eq.enabled {
        ((eq.treble_db - eq.bass_db) / (2.0 * TILT_DB)).clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

pub fn set_tone_tilt(eq: &mut EqSettings, tilt: f32) {
    let tilt = tilt.clamp(-1.0, 1.0);
    eq.bass_db = -tilt * TILT_DB;
    eq.treble_db = tilt * TILT_DB;
    if tilt.abs() > PAD_OFF {
        eq.enabled = true;
    }
}

/// Pad Y: 0 (dry) .. 1 (full echo). Reads 0 while the echo is off.
pub fn echo_amount(echo: &EchoSettings) -> f32 {
    if echo.enabled {
        (echo.mix / ECHO_MAX_MIX).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

pub fn set_echo_amount(echo: &mut EchoSettings, amount: f32) {
    let amount = amount.clamp(0.0, 1.0);
    echo.enabled = amount > PAD_OFF;
    if echo.enabled {
        echo.mix = amount * ECHO_MAX_MIX;
    }
}

pub struct StripView<'a> {
    pub index: usize,
    pub settings: &'a mut StripSettings,
    pub meters: &'a Meters,
    pub any_solo: bool,
    /// Whether this strip's fine-tune window is open.
    pub fine_tune_open: &'a mut bool,
    pub geo: Geometry,
    /// Fader height for this frame, derived from the row height.
    pub fader_height: f32,
    /// Apps playing into this input's cable (virtual inputs only).
    pub apps: &'a [String],
    /// What the app list says when it is empty.
    pub apps_empty: &'a str,
}

impl StripView<'_> {
    /// Console order, top to bottom: colour pad, knobs with effect LEDs, pan, then the fader with
    /// its meter and the two button columns (routes; mono / solo / mute).
    pub fn show(&mut self, ui: &mut Ui) {
        let silenced = self.settings.mute || (self.any_solo && !self.settings.solo);
        widgets::section(ui, "Colour");
        self.colour_pad(ui);
        widgets::section(ui, "Effects");
        self.knobs_and_leds(ui);
        pan_row(ui, &mut self.settings.pan, self.geo.inner);
        widgets::section(ui, "Level and send to");
        let geo = self.geo;
        ui.horizontal_top(|ui| {
            ui.scope(|ui| {
                if silenced {
                    ui.set_opacity(0.45);
                }
                widgets::fader(ui, &mut self.settings.gain_db, self.fader_height);
                widgets::meter(ui, self.meters.strips[self.index].load(), self.fader_height, widgets::METER_WIDTH);
            });
            ui.vertical(|ui| {
                ui.set_width(geo.side_button.x);
                routing_column(ui, &mut self.settings.routing, self.index, geo);
            });
            ui.vertical(|ui| {
                ui.set_width(geo.side_button.x);
                state_column(ui, self.settings, geo, self.fine_tune_open);
            });
            widgets::app_list(ui, geo.strip_apps, self.apps, self.apps_empty);
        });
        self.fine_tune_window(ui.ctx());
    }

    fn colour_pad(&mut self, ui: &mut Ui) {
        let s = &mut self.settings;
        let mut tilt = tone_tilt(&s.eq);
        let mut echo = echo_amount(&s.echo);
        if widgets::xy_pad(ui, &mut tilt, &mut echo, self.geo.xy_pad, ["Lo", "Hi", "ECHO"], "Colour pad: left is warm, right is bright; up adds echo. Double-click resets.") {
            set_tone_tilt(&mut s.eq, tilt);
            set_echo_amount(&mut s.echo, echo);
        }
    }

    fn knobs_and_leds(&mut self, ui: &mut Ui) {
        let geo = self.geo;
        let s = &mut self.settings;
        ui.horizontal_top(|ui| {
            let mut comp = comp_amount(&s.comp);
            if widgets::knob(ui, &mut comp, KNOB_RANGE, 0.0, "COMP", "Even out loud and quiet moments.") {
                set_comp_amount(&mut s.comp, comp);
            }
            let mut gate = gate_amount(&s.gate);
            if widgets::knob(ui, &mut gate, KNOB_RANGE, 0.0, "GATE", "Close the mic when you are not talking.") {
                set_gate_amount(&mut s.gate, gate);
            }
            ui.vertical(|ui| {
                ui.add_space(widgets::SECTION_GAP);
                ui.horizontal(|ui| {
                    widgets::led(ui, &mut s.denoise, "DENOISE", COLOR_ACTIVE, geo.fx_led, "Remove fans, hum and keyboard noise (RNNoise).");
                    widgets::led(ui, &mut s.eq.enabled, "EQ", COLOR_ACTIVE, geo.fx_led, "Shape the tone: bass, mid, treble, low cut.");
                });
                ui.horizontal(|ui| {
                    widgets::led(ui, &mut s.echo.enabled, "ECHO", COLOR_ACTIVE, geo.fx_led, "Repeating delay effect.");
                    widgets::led(ui, &mut s.reverb.enabled, "REVERB", COLOR_ACTIVE, geo.fx_led, "Room ambience.");
                });
            });
        });
    }

    fn fine_tune_window(&mut self, ctx: &egui::Context) {
        fine_tune_window(ctx, self.index, self.settings, self.meters, self.fine_tune_open);
    }
}

/// Every effect parameter of one strip, in a floating window that never grows the strip.
pub fn fine_tune_window(ctx: &egui::Context, index: usize, s: &mut StripSettings, meters: &Meters, open: &mut bool) {
    if !*open {
        return;
    }
    {
        egui::Window::new(format!("Fine-tune · {}", crate::strip_name(index)))
            .id(egui::Id::new(("fine-tune", index)))
            .open(open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.spacing_mut().slider_width = widgets::FINE_TUNE_SLIDER_WIDTH;
                widgets::section(ui, "Gate");
                if !s.gate.enabled {
                    widgets::hint(ui, "Off. Turn the GATE knob to enable.");
                }
                ui.add(egui::Slider::new(&mut s.gate.threshold_db, -80.0..=0.0).text("open at dB"));
                ui.add(egui::Slider::new(&mut s.gate.release_ms, 10.0..=1000.0).text("release ms"));
                let open = meters.gate_open[index].load(std::sync::atomic::Ordering::Relaxed);
                widgets::hint(ui, if s.gate.enabled && !open { "Gate closed" } else { "Gate open" });

                widgets::section(ui, "EQ");
                widgets::led(ui, &mut s.eq.low_cut, "LOW CUT 80 Hz", COLOR_ACTIVE, egui::Vec2::new(widgets::FINE_TUNE_SLIDER_WIDTH, widgets::LED_HEIGHT), "Remove rumble and desk thumps.");
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

/// The pan slider shares its row with the "Fine-tune…" button.
pub fn pan_slider_width(inner: f32) -> f32 {
    (inner - PAN_ROW_FIXED).max(40.0)
}

/// Room the "pan" label and the "Fine-tune…" button take in the pan row.
const PAN_ROW_FIXED: f32 = 132.0;

/// Room the "pan" label alone takes when the slider has the row to itself.
const PAN_LABEL_WIDTH: f32 = 36.0;

/// A pan slider that really resets to centre on double-click (egui's slider does not by itself).
pub fn pan_row(ui: &mut Ui, pan: &mut f32, inner: f32) {
    ui.spacing_mut().slider_width = inner - PAN_LABEL_WIDTH;
    let response = ui.add(egui::Slider::new(pan, -1.0..=1.0).show_value(false).text("pan")).on_hover_text("Left / right balance. Double-click to centre.");
    if response.double_clicked() {
        *pan = 0.0;
    }
}

/// Routes stacked beside the fader: A1.. in green, then B1.. in blue.
pub fn routing_column(ui: &mut Ui, routing: &mut [bool; NUM_BUSES], strip: usize, geo: Geometry) {
    for (b, on) in routing.iter_mut().enumerate() {
        let (color, tip) = if b < NUM_HW_BUSES { (COLOR_ACTIVE, TIP_HARDWARE_ROUTE) } else { (COLOR_VIRTUAL, TIP_VIRTUAL_ROUTE) };
        ui.push_id((strip, b), |ui| widgets::led(ui, on, &bus_name(b), color, geo.side_button, tip));
    }
}

/// MONO / SOLO / MUTE and the fine-tune opener, stacked beside the routes.
pub fn state_column(ui: &mut Ui, s: &mut StripSettings, geo: Geometry, fine_tune_open: &mut bool) {
    widgets::led(ui, &mut s.mono, "MONO", COLOR_ACTIVE, geo.side_button, "Sum left and right into the centre. Use for a single microphone.");
    widgets::led(ui, &mut s.solo, "SOLO", COLOR_SOLO, geo.side_button, "Hear only soloed strips.");
    widgets::led(ui, &mut s.mute, "MUTE", COLOR_MUTE, geo.side_button, "Silence this strip on every bus.");
    widgets::led(ui, fine_tune_open, "FINE-TUNE", widgets::COLOR_HEADING, geo.fine_tune_button, "All effect parameters in a separate window.");
}

/// MONO / SOLO / MUTE.
pub fn state_row(ui: &mut Ui, s: &mut StripSettings, geo: Geometry) {
    ui.horizontal(|ui| {
        widgets::led(ui, &mut s.mono, "MONO", COLOR_ACTIVE, geo.led_triple, "Sum left and right into the centre. Use for a single microphone.");
        widgets::led(ui, &mut s.solo, "SOLO", COLOR_SOLO, geo.led_triple, "Hear only soloed strips.");
        widgets::led(ui, &mut s.mute, "MUTE", COLOR_MUTE, geo.led_triple, "Silence this strip on every bus.");
    });
}

/// ECHO / REVERB.
pub fn fx_row(ui: &mut Ui, s: &mut StripSettings, geo: Geometry) {
    ui.horizontal(|ui| {
        widgets::led(ui, &mut s.echo.enabled, "ECHO", COLOR_ACTIVE, geo.led_pair, "Repeating delay effect.");
        widgets::led(ui, &mut s.reverb.enabled, "REVERB", COLOR_ACTIVE, geo.led_pair, "Room ambience.");
    });
}

/// Two rows of LEDs: hardware outs A1..A3 in green, then virtual outs B1..B2 in blue.
pub fn routing_rows(ui: &mut Ui, routing: &mut [bool; NUM_BUSES], strip: usize, geo: Geometry) {
    let (hardware, virtual_buses) = routing.split_at_mut(NUM_HW_BUSES);
    ui.horizontal(|ui| {
        for (b, on) in hardware.iter_mut().enumerate() {
            ui.push_id((strip, b), |ui| {
                widgets::led(ui, on, &bus_name(b), COLOR_ACTIVE, geo.led_route_hardware, TIP_HARDWARE_ROUTE)
            });
        }
    });
    ui.horizontal(|ui| {
        for (offset, on) in virtual_buses.iter_mut().enumerate() {
            let b = NUM_HW_BUSES + offset;
            ui.push_id((strip, b), |ui| {
                widgets::led(ui, on, &bus_name(b), COLOR_VIRTUAL, geo.led_route_virtual, TIP_VIRTUAL_ROUTE)
            });
        }
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
        assert!(!c.enabled);
    }

    #[test]
    fn comp_amount_clamps_values_above_ten_on_read_back() {
        let c = CompressorSettings { enabled: true, threshold_db: -100.0, ..Default::default() };
        assert_eq!(comp_amount(&c), 10.0);
    }

    #[test]
    fn gate_amount_clamps_to_ten_when_threshold_is_above_minus_20_db() {
        let g = GateSettings { enabled: true, threshold_db: 0.0, ..Default::default() };
        assert_eq!(gate_amount(&g), 10.0);
    }

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

    #[test]
    fn pad_tilt_moves_bass_and_treble_in_opposite_directions_and_enables_eq() {
        let mut eq = EqSettings::default();
        set_tone_tilt(&mut eq, 0.5);
        assert!(eq.enabled);
        assert_eq!(eq.bass_db, -4.0);
        assert_eq!(eq.treble_db, 4.0);
        assert!((tone_tilt(&eq) - 0.5).abs() < 1e-6);
        set_tone_tilt(&mut eq, -1.0);
        assert_eq!(eq.bass_db, 8.0);
        assert_eq!(eq.treble_db, -8.0);
    }

    #[test]
    fn pad_echo_enables_and_scales_the_mix_and_zero_turns_it_off() {
        let mut echo = EchoSettings::default();
        set_echo_amount(&mut echo, 1.0);
        assert!(echo.enabled);
        assert!((echo.mix - ECHO_MAX_MIX).abs() < 1e-6);
        assert!((echo_amount(&echo) - 1.0).abs() < 1e-6);
        set_echo_amount(&mut echo, 0.0);
        assert!(!echo.enabled);
        assert_eq!(echo_amount(&echo), 0.0);
    }

    #[test]
    fn set_tone_tilt_at_exactly_the_off_threshold_stays_off() {
        let mut eq = EqSettings::default();
        set_tone_tilt(&mut eq, PAD_OFF);
        assert!(!eq.enabled);
        assert_eq!(eq.bass_db, -PAD_OFF * TILT_DB);
        assert_eq!(eq.treble_db, PAD_OFF * TILT_DB);
    }

    #[test]
    fn pan_slider_width_never_drops_below_its_floor() {
        // Even a column so narrow it goes negative once the fixed "Fine-tune…" row is
        // subtracted must not hand the slider a negative or zero width.
        for inner in [-100.0, 0.0, 40.0, 100.0, PAN_ROW_FIXED, PAN_ROW_FIXED + 39.0, 1000.0] {
            let w = pan_slider_width(inner);
            assert!(w >= 40.0, "pan slider width {w} fell below its floor for inner={inner}");
        }
        // Above the floor, it tracks the available width exactly.
        assert_eq!(pan_slider_width(PAN_ROW_FIXED + 100.0), 100.0);
    }

    #[test]
    fn set_echo_amount_clamps_values_above_one() {
        let mut echo = EchoSettings::default();
        set_echo_amount(&mut echo, 5.0);
        assert!(echo.enabled);
        assert!((echo.mix - ECHO_MAX_MIX).abs() < 1e-6);
        assert!((echo_amount(&echo) - 1.0).abs() < 1e-6);
    }
}
