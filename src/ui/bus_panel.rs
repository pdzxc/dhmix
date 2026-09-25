//! One output bus: BASS and TREBLE knobs over the LIMIT knob and MUTE, the fader and meter, and
//! beside them the apps listening to this output.

use super::widgets::{self, Geometry, COLOR_MUTE};
use crate::engine::settings::{LIMIT_DB_RANGE, LIMIT_DEFAULT_DB};
use crate::engine::{BusSettings, Meters};
use egui::Ui;

const TONE_RANGE: std::ops::RangeInclusive<f32> = -12.0..=12.0;

pub struct BusView<'a> {
    pub index: usize,
    pub settings: &'a mut BusSettings,
    pub meters: &'a Meters,
    pub geo: Geometry,
    pub fader_height: f32,
    /// Apps recording from this output's cable (virtual buses only).
    pub apps: &'a [String],
    /// What the app list says when it is empty.
    pub apps_empty: &'a str,
}

impl BusView<'_> {
    pub fn show(&mut self, ui: &mut Ui) {
        let geo = self.geo;
        widgets::section(ui, "Tone and level");
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(geo.bus_left);
                ui.horizontal(|ui| {
                    widgets::knob(ui, &mut self.settings.bass_db, TONE_RANGE, 0.0, "BASS", "Low shelf at 100 Hz, in dB.");
                    widgets::knob(ui, &mut self.settings.treble_db, TONE_RANGE, 0.0, "TREBLE", "High shelf at 8 kHz, in dB.");
                });
                ui.horizontal(|ui| {
                    widgets::knob(ui, &mut self.settings.limit_db, LIMIT_DB_RANGE, LIMIT_DEFAULT_DB, "LIMIT", "Ceiling in dB: peaks above it are held down. Full (0) only stops clipping; turn down to tame a loud output.");
                    widgets::led(ui, &mut self.settings.mute, "MUTE", COLOR_MUTE, geo.bus_button, "Silence this output.");
                });
            });
            ui.scope(|ui| {
                if self.settings.mute {
                    ui.set_opacity(0.45);
                }
                widgets::fader(ui, &mut self.settings.gain_db, self.fader_height);
                widgets::meter(ui, self.meters.buses[self.index].load(), self.fader_height, widgets::METER_WIDTH);
            });
            widgets::app_list(ui, geo.bus_apps, self.apps, self.apps_empty);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lays out a whole output column (header, device picker, bus) the way `App::buses_row`
    /// does and returns how tall its content came out beyond the fader.
    fn rendered_fixed_height(fader_height: f32) -> f32 {
        let geo = Geometry::for_column(widgets::column_width(widgets::WINDOW_SIZE.x - 2.0 * widgets::SECTION_GAP, crate::NUM_BUSES));
        let meters = Meters::default();
        let settings = std::cell::RefCell::new(BusSettings::default());
        let content_height = std::cell::Cell::new(0.0);
        widgets::run_themed_test_ui(|ui| {
            widgets::panel(ui, geo.inner + 2.0 * widgets::PANEL_PADDING, 2000.0, |ui| {
                widgets::panel_header(ui, "Hardware out", Some(("A1", widgets::bus_color(0))), None);
                widgets::device_combo(ui, "out", &mut None, &[], "Choose speakers / headphones…", geo.inner);
                BusView { index: 0, settings: &mut settings.borrow_mut(), meters: &meters, geo, fader_height, apps: &[], apps_empty: super::super::app::APPS_EMPTY_VIRTUAL_OUT }.show(ui);
                content_height.set(ui.min_rect().height());
            });
        });
        content_height.get() + 2.0 * widgets::PANEL_PADDING - fader_height
    }

    /// `OUTPUT_FIXED_HEIGHT` is what `App` subtracts from the row to size the fader; if the bus
    /// renders taller than that, the fader's value readout is clipped by the panel.
    #[test]
    fn output_fixed_height_matches_the_rendered_bus() {
        let measured = rendered_fixed_height(widgets::FADER_MIN_HEIGHT);
        assert!(
            (measured - widgets::OUTPUT_FIXED_HEIGHT).abs() < 1.0,
            "the bus renders {measured} px beyond its fader; set OUTPUT_FIXED_HEIGHT to that"
        );
        assert!((rendered_fixed_height(200.0) - measured).abs() < 1e-3, "the fixed part does not depend on the fader height");
    }
}
