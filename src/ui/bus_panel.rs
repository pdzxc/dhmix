//! One output bus: tone knobs, then the fader with a wide meter and the limit / mute column, so
//! the whole column width is used.

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
}

impl BusView<'_> {
    pub fn show(&mut self, ui: &mut Ui) {
        let geo = self.geo;
        widgets::section(ui, "Tone and level");
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                widgets::knob(ui, &mut self.settings.bass_db, TONE_RANGE, 0.0, "BASS", "Low shelf at 100 Hz, in dB.");
                widgets::knob(ui, &mut self.settings.treble_db, TONE_RANGE, 0.0, "TREBLE", "High shelf at 8 kHz, in dB.");
            });
            ui.scope(|ui| {
                if self.settings.mute {
                    ui.set_opacity(0.45);
                }
                widgets::fader(ui, &mut self.settings.gain_db, self.fader_height);
                widgets::meter(ui, self.meters.buses[self.index].load(), self.fader_height, geo.bus_meter);
            });
            ui.vertical(|ui| {
                ui.set_width(geo.bus_button.x);
                widgets::led(ui, &mut self.settings.limiter, "LIMIT", COLOR_ACTIVE, geo.bus_button, "Stop peaks from clipping. Leave on unless you know why.");
                widgets::led(ui, &mut self.settings.mute, "MUTE", COLOR_MUTE, geo.bus_button, "Silence this output.");
            });
        });
    }
}
