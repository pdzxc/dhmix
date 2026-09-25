//! One output bus column: device, mute, limiter, tone, fader and meter.

use super::widgets::{self, COLOR_ACTIVE, COLOR_MUTE};
use crate::engine::{BusSettings, Meters};
use egui::Ui;

pub struct BusView<'a> {
    pub index: usize,
    pub settings: &'a mut BusSettings,
    pub meters: &'a Meters,
}

impl BusView<'_> {
    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            widgets::toggle(ui, &mut self.settings.mute, "MUTE", COLOR_MUTE);
            widgets::toggle(ui, &mut self.settings.limiter, "LIMIT", COLOR_ACTIVE);
        });
        ui.spacing_mut().slider_width = widgets::DETAIL_SLIDER_WIDTH;
        ui.add(egui::Slider::new(&mut self.settings.bass_db, -12.0..=12.0).text("bass"));
        ui.add(egui::Slider::new(&mut self.settings.treble_db, -12.0..=12.0).text("treble"));
        ui.horizontal(|ui| {
            widgets::fader(ui, &mut self.settings.gain_db, -60.0..=12.0);
            widgets::meter(ui, self.meters.buses[self.index].load());
        });
    }
}
