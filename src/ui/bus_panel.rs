//! One output bus: mute, limiter and tone knobs on the left; ruler fader and meter on the right.

use super::widgets::{self, COLOR_ACTIVE, COLOR_MUTE, LED_TRIPLE};
use crate::engine::{BusSettings, Meters};
use egui::Ui;

const TONE_RANGE: std::ops::RangeInclusive<f32> = -12.0..=12.0;

pub struct BusView<'a> {
    pub index: usize,
    pub settings: &'a mut BusSettings,
    pub meters: &'a Meters,
}

impl BusView<'_> {
    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(widgets::STRIP_LEFT_WIDTH);
                widgets::section(ui, "Tone");
                ui.horizontal(|ui| {
                    widgets::knob(ui, &mut self.settings.bass_db, TONE_RANGE, 0.0, "BASS", "Low shelf at 100 Hz, in dB.");
                    widgets::knob(ui, &mut self.settings.treble_db, TONE_RANGE, 0.0, "TREBLE", "High shelf at 8 kHz, in dB.");
                });
                ui.add_space(widgets::SECTION_GAP);
                ui.horizontal(|ui| {
                    widgets::led(ui, &mut self.settings.limiter, "LIMIT", COLOR_ACTIVE, LED_TRIPLE, "Stop peaks from clipping. Leave on unless you know why.");
                    widgets::led(ui, &mut self.settings.mute, "MUTE", COLOR_MUTE, LED_TRIPLE, "Silence this output.");
                });
            });
            ui.scope(|ui| {
                if self.settings.mute {
                    ui.set_opacity(0.45);
                }
                widgets::fader(ui, &mut self.settings.gain_db);
                widgets::meter(ui, self.meters.buses[self.index].load());
            });
        });
    }
}
