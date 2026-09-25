//! One input strip, top to bottom: the colour pad beside the COMP / GATE knobs with the pan
//! control under them, a row of effect LEDs, the A / B send-to rows, then the ruler fader with
//! its meter, the MONO / SOLO / MUTE column and the app list. Detailed effect sliders open in a
//! floating window so the strip never grows.

use super::widgets::{self, Geometry, COLOR_ACTIVE, COLOR_MUTE, COLOR_SOLO};
use crate::dsp::compressor::CompressorSettings;
use crate::dsp::echo::EchoSettings;
use crate::dsp::eq::EqSettings;
use crate::dsp::gate::GateSettings;
use crate::engine::{Meters, StripSettings};
use crate::{bus_name, NUM_BUSES, NUM_HW_BUSES};
use egui::Ui;

const TIP_HARDWARE_ROUTE: &str = "Hardware out: speakers or headphones";
const TIP_VIRTUAL_ROUTE: &str = "Virtual out: what OBS, Discord or a call hears";
/// What each send-to row is: the A row monitors (you hear it), the B row broadcasts (your
/// stream, recording or call hears it).
const HINT_HARDWARE_ROUTE: &str = "Monitor";
const HINT_VIRTUAL_ROUTE: &str = "Broadcast";
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
/// The bottom share of the pad where echo stays off, so a nudge upward while setting the tone
/// does not add echo; the pad draws a line there. Echo ramps from that line to the top.
pub const ECHO_DEAD_ZONE: f32 = 0.25;

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

/// Echo amount 0 (dry) .. 1 (full echo). Reads 0 while the echo is off.
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

/// Pad Y (0 bottom .. 1 top) to echo amount: nothing inside the dead zone, then a ramp to 1.
pub fn pad_y_to_echo(y: f32) -> f32 {
    ((y - ECHO_DEAD_ZONE) / (1.0 - ECHO_DEAD_ZONE)).clamp(0.0, 1.0)
}

/// Echo amount back to the pad Y that shows it; 0 sits on the dead-zone line.
pub fn echo_to_pad_y(amount: f32) -> f32 {
    if amount <= 0.0 {
        0.0
    } else {
        ECHO_DEAD_ZONE + amount.clamp(0.0, 1.0) * (1.0 - ECHO_DEAD_ZONE)
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
    /// Console order, top to bottom: colour pad beside the knobs and pan, effect LEDs, the send-to
    /// rows, then the fader with its meter, the mono / solo / mute column and the app list.
    pub fn show(&mut self, ui: &mut Ui) {
        let silenced = self.settings.mute || (self.any_solo && !self.settings.solo);
        let geo = self.geo;
        widgets::section(ui, "Audio effects");
        ui.horizontal_top(|ui| {
            self.colour_pad(ui);
            ui.vertical(|ui| {
                ui.set_width(widgets::KNOB_BLOCK_WIDTH);
                self.knobs(ui);
                widgets::pan(ui, &mut self.settings.pan, widgets::KNOB_BLOCK_WIDTH);
            });
        });
        fx_row(ui, self.settings, geo.fx_led);
        widgets::section(ui, "Send to");
        routing_rows(ui, &mut self.settings.routing, self.index, geo);
        widgets::section(ui, "Level");
        ui.horizontal_top(|ui| {
            ui.scope(|ui| {
                if silenced {
                    ui.set_opacity(0.45);
                }
                widgets::fader(ui, &mut self.settings.gain_db, self.fader_height);
                widgets::meter(ui, self.meters.strips[self.index].load(), self.fader_height, widgets::METER_WIDTH);
            });
            widgets::level_column(ui, geo.side_button.x, |ui| state_column(ui, self.settings, geo, self.fine_tune_open));
            widgets::app_list(ui, geo.strip_apps, self.apps, self.apps_empty);
        });
        self.fine_tune_window(ui.ctx());
    }

    fn colour_pad(&mut self, ui: &mut Ui) {
        let s = &mut self.settings;
        let mut tilt = tone_tilt(&s.eq);
        let mut y = echo_to_pad_y(echo_amount(&s.echo));
        if widgets::xy_pad(ui, &mut tilt, &mut y, self.geo.xy_pad, ["Lo", "Hi", "ECHO"], Some(ECHO_DEAD_ZONE), "Tone and echo: left is warm, right is bright. Echo starts above the line and grows towards the top. Double-click resets.") {
            set_tone_tilt(&mut s.eq, tilt);
            set_echo_amount(&mut s.echo, pad_y_to_echo(y));
        }
    }

    fn knobs(&mut self, ui: &mut Ui) {
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
                widgets::slider(ui, egui::Slider::new(&mut s.gate.threshold_db, -80.0..=0.0).text("open at dB"));
                widgets::slider(ui, egui::Slider::new(&mut s.gate.release_ms, 10.0..=1000.0).text("release ms"));
                let open = meters.gate_open[index].load(std::sync::atomic::Ordering::Relaxed);
                widgets::hint(ui, if s.gate.enabled && !open { "Gate closed" } else { "Gate open" });

                widgets::section(ui, "EQ");
                widgets::led(ui, &mut s.eq.low_cut, "LOW CUT 80 Hz", COLOR_ACTIVE, egui::Vec2::new(widgets::FINE_TUNE_SLIDER_WIDTH, widgets::LED_HEIGHT), "Remove rumble and desk thumps.");
                widgets::slider(ui, egui::Slider::new(&mut s.eq.bass_db, -12.0..=12.0).text("bass dB"));
                widgets::slider(ui, egui::Slider::new(&mut s.eq.mid_db, -12.0..=12.0).text("mid dB"));
                widgets::slider(ui, egui::Slider::new(&mut s.eq.treble_db, -12.0..=12.0).text("treble dB"));

                widgets::section(ui, "Compressor");
                if !s.comp.enabled {
                    widgets::hint(ui, "Off. Turn the COMP knob to enable.");
                }
                widgets::slider(ui, egui::Slider::new(&mut s.comp.threshold_db, -60.0..=0.0).text("threshold dB"));
                widgets::slider(ui, egui::Slider::new(&mut s.comp.ratio, 1.0..=20.0).text("ratio"));
                widgets::slider(ui, egui::Slider::new(&mut s.comp.attack_ms, 0.1..=100.0).logarithmic(true).text("attack ms"));
                widgets::slider(ui, egui::Slider::new(&mut s.comp.release_ms, 10.0..=1000.0).logarithmic(true).text("release ms"));
                widgets::slider(ui, egui::Slider::new(&mut s.comp.makeup_db, 0.0..=24.0).text("makeup dB"));
                widgets::hint(ui, format!("Reducing {:.1} dB", meters.reduction(index)));

                widgets::section(ui, "Echo");
                widgets::slider(ui, egui::Slider::new(&mut s.echo.time_ms, 20.0..=2000.0).text("time ms"));
                widgets::slider(ui, egui::Slider::new(&mut s.echo.feedback, 0.0..=0.95).text("feedback"));
                widgets::slider(ui, egui::Slider::new(&mut s.echo.mix, 0.0..=1.0).text("mix"));

                widgets::section(ui, "Reverb");
                widgets::slider(ui, egui::Slider::new(&mut s.reverb.size, 0.0..=1.0).text("size"));
                widgets::slider(ui, egui::Slider::new(&mut s.reverb.damping, 0.0..=1.0).text("damping"));
                widgets::slider(ui, egui::Slider::new(&mut s.reverb.mix, 0.0..=1.0).text("mix"));
            });
    }
}

/// MONO / SOLO / MUTE and the fine-tune opener, stacked beside the fader.
pub fn state_column(ui: &mut Ui, s: &mut StripSettings, geo: Geometry, fine_tune_open: &mut bool) {
    widgets::led(ui, &mut s.mono, "MONO", COLOR_ACTIVE, geo.side_button, "Sum left and right into the centre. Use for a single microphone.");
    widgets::led(ui, &mut s.solo, "SOLO", COLOR_SOLO, geo.side_button, "Hear only soloed strips.");
    widgets::led(ui, &mut s.mute, "MUTE", COLOR_MUTE, geo.side_button, "Silence this strip on every bus.");
    widgets::led(ui, fine_tune_open, "FINE-TUNE", widgets::COLOR_HEADING, geo.fine_tune_button, "All effect parameters in a separate window.");
}

/// DENOISE / EQ / ECHO / REVERB in one row of `led`-sized LEDs.
pub fn fx_row(ui: &mut Ui, s: &mut StripSettings, led: egui::Vec2) {
    ui.horizontal(|ui| {
        widgets::led(ui, &mut s.denoise, "DENOISE", COLOR_ACTIVE, led, "Noise suppression, like Discord's: a neural filter (RNNoise) removes fans, hum and keyboard noise while keeping your voice.");
        widgets::led(ui, &mut s.eq.enabled, "EQ", COLOR_ACTIVE, led, "Shape the tone: bass, mid, treble, low cut.");
        widgets::led(ui, &mut s.echo.enabled, "ECHO", COLOR_ACTIVE, led, "Repeating delay effect.");
        widgets::led(ui, &mut s.reverb.enabled, "REVERB", COLOR_ACTIVE, led, "Room ambience.");
    });
}

/// Two rows of small LEDs, each ending in what it does: hardware outs A1..A3 in green
/// ("Monitor"), then virtual outs B1..B2 in blue ("Broadcast").
pub fn routing_rows(ui: &mut Ui, routing: &mut [bool; NUM_BUSES], strip: usize, geo: Geometry) {
    let (hardware, virtual_buses) = routing.split_at_mut(NUM_HW_BUSES);
    ui.horizontal(|ui| {
        for (b, on) in hardware.iter_mut().enumerate() {
            ui.push_id((strip, b), |ui| widgets::led(ui, on, &bus_name(b), widgets::bus_color(b), geo.route_led, TIP_HARDWARE_ROUTE));
        }
        widgets::row_hint(ui, HINT_HARDWARE_ROUTE);
    });
    ui.horizontal(|ui| {
        for (offset, on) in virtual_buses.iter_mut().enumerate() {
            let b = NUM_HW_BUSES + offset;
            ui.push_id((strip, b), |ui| widgets::led(ui, on, &bus_name(b), widgets::bus_color(b), geo.route_led, TIP_VIRTUAL_ROUTE));
        }
        widgets::row_hint(ui, HINT_VIRTUAL_ROUTE);
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

    /// Lays out a whole input column (header, device picker, strip) the way `App::strips_row`
    /// does and returns how tall its content came out beyond the fader.
    fn rendered_fixed_height(fader_height: f32) -> f32 {
        let geo = Geometry::for_column(widgets::column_width(widgets::WINDOW_SIZE.x - 2.0 * widgets::SECTION_GAP, crate::NUM_BUSES));
        let meters = Meters::default();
        // The test harness takes an `Fn` closure, so the strip's state lives in cells.
        let settings = std::cell::RefCell::new(StripSettings::default());
        let open = std::cell::Cell::new(false);
        let content_height = std::cell::Cell::new(0.0);
        widgets::run_themed_test_ui(|ui| {
            widgets::panel(ui, geo.inner + 2.0 * widgets::PANEL_PADDING, 2000.0, |ui| {
                widgets::panel_header(ui, "Hardware input 1", None, None);
                widgets::device_combo(ui, "in", &mut None, &[], "Choose a microphone…", geo.inner);
                let mut fine_tune_open = open.get();
                StripView { index: 0, settings: &mut settings.borrow_mut(), meters: &meters, any_solo: false, fine_tune_open: &mut fine_tune_open, geo, fader_height, apps: &[], apps_empty: super::super::app::APPS_EMPTY_VIRTUAL_IN }.show(ui);
                open.set(fine_tune_open);
                content_height.set(ui.min_rect().height());
            });
        });
        content_height.get() + 2.0 * widgets::PANEL_PADDING - fader_height
    }

    /// Each send-to row (LEDs, the gap, the hint text) must fit the column at the fixed window
    /// size, or the hint is clipped by the panel edge.
    #[test]
    fn send_to_rows_with_their_hints_fit_the_column() {
        let geo = Geometry::for_column(widgets::column_width(widgets::WINDOW_SIZE.x - 2.0 * widgets::SECTION_GAP, crate::NUM_BUSES));
        let routing = std::cell::Cell::new([false; NUM_BUSES]);
        let width = std::cell::Cell::new(0.0);
        widgets::run_themed_test_ui(|ui| {
            ui.vertical(|ui| {
                ui.set_width(geo.inner);
                let mut r = routing.get();
                routing_rows(ui, &mut r, 0, geo);
                routing.set(r);
                width.set(ui.min_rect().width());
            });
        });
        assert!(width.get() <= geo.inner + 1e-3, "send-to rows are {} px wide in a {} px column", width.get(), geo.inner);
    }

    /// `PlayerPanel::show` gives `routing_rows` only `geo.left` (the pad grid's width), not the
    /// full column, since the fader and meter take the rest beside it. The send-to rows with
    /// their hints must fit there too, or the hint is clipped by the floating window's edge.
    #[test]
    fn send_to_rows_fit_the_player_windows_left_column() {
        let geo = Geometry::for_column(widgets::PLAYER_WINDOW_WIDTH + 2.0 * widgets::PANEL_PADDING);
        let routing = std::cell::Cell::new([false; NUM_BUSES]);
        let width = std::cell::Cell::new(0.0);
        widgets::run_themed_test_ui(|ui| {
            ui.vertical(|ui| {
                ui.set_width(geo.left);
                let mut r = routing.get();
                routing_rows(ui, &mut r, 0, geo);
                routing.set(r);
                width.set(ui.min_rect().width());
            });
        });
        assert!(width.get() <= geo.left + 1e-3, "send-to rows are {} px wide in the Player window's {} px left column", width.get(), geo.left);
    }

    /// `INPUT_FIXED_HEIGHT` is what `App` subtracts from the row to size the fader; if the strip
    /// renders taller than that, the fader's value readout is clipped by the panel.
    #[test]
    fn input_fixed_height_matches_the_rendered_strip() {
        let measured = rendered_fixed_height(widgets::FADER_MIN_HEIGHT);
        assert!(
            (measured - widgets::INPUT_FIXED_HEIGHT).abs() < 1.0,
            "the strip renders {measured} px beyond its fader; set INPUT_FIXED_HEIGHT to that"
        );
        assert!((rendered_fixed_height(200.0) - measured).abs() < 1e-3, "the fixed part does not depend on the fader height");
    }

    #[test]
    fn pad_dead_zone_keeps_echo_off_and_round_trips_above_it() {
        assert_eq!(pad_y_to_echo(0.0), 0.0);
        assert_eq!(pad_y_to_echo(ECHO_DEAD_ZONE * 0.9), 0.0, "a nudge inside the dead zone adds no echo");
        assert_eq!(pad_y_to_echo(ECHO_DEAD_ZONE), 0.0, "the line itself is still dry");
        assert!((pad_y_to_echo(1.0) - 1.0).abs() < 1e-6);
        assert_eq!(echo_to_pad_y(0.0), 0.0, "dry sits at the bottom, not on the line");
        for amount in [0.1, 0.5, 1.0] {
            assert!((pad_y_to_echo(echo_to_pad_y(amount)) - amount).abs() < 1e-6, "amount {amount} round-trips through the pad");
        }
        let mut echo = EchoSettings::default();
        set_echo_amount(&mut echo, pad_y_to_echo(0.2));
        assert!(!echo.enabled, "a dot low on the pad leaves the echo off");
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
