//! One output bus: tone knobs with LIMIT / MUTE beneath them, the fader and meter, and beside
//! them the apps listening to this output.

use super::widgets::{self, Geometry, COLOR_ACTIVE, COLOR_MUTE};
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
                    widgets::led(ui, &mut self.settings.limiter, "LIMIT", COLOR_ACTIVE, geo.bus_button, "Stop peaks from clipping. Leave on unless you know why.");
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
