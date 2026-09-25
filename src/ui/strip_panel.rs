//! One input strip column: device, routing, mono/solo/mute, FX, fader and meter.

use super::widgets::{self, COLOR_ACTIVE, COLOR_MUTE, COLOR_SOLO};
use crate::engine::{Meters, StripSettings};
use crate::{bus_name, NUM_BUSES, NUM_HW_BUSES};
use egui::Ui;

pub struct StripView<'a> {
    pub index: usize,
    pub settings: &'a mut StripSettings,
    pub meters: &'a Meters,
    pub any_solo: bool,
}

impl StripView<'_> {
    pub fn show(&mut self, ui: &mut Ui) {
        routing_rows(ui, &mut self.settings.routing, self.index);
        ui.horizontal(|ui| {
            widgets::toggle(ui, &mut self.settings.mono, "MONO", COLOR_ACTIVE);
            widgets::toggle(ui, &mut self.settings.solo, "SOLO", COLOR_SOLO);
            widgets::toggle(ui, &mut self.settings.mute, "MUTE", COLOR_MUTE);
        });
        if self.any_solo && !self.settings.solo {
            widgets::hint(ui, "silent: another strip is soloed");
        }
        self.fx_toggles(ui);
        self.fx_details(ui);
        ui.horizontal(|ui| {
            widgets::fader(ui, &mut self.settings.gain_db, -60.0..=12.0);
            widgets::meter(ui, self.meters.strips[self.index].load());
        });
        ui.add(egui::Slider::new(&mut self.settings.pan, -1.0..=1.0).show_value(false).text("pan"));
    }

    fn fx_toggles(&mut self, ui: &mut Ui) {
        let s = &mut self.settings;
        ui.horizontal_wrapped(|ui| {
            widgets::toggle(ui, &mut s.denoise, "DENOISE", COLOR_ACTIVE);
            widgets::toggle(ui, &mut s.gate.enabled, "GATE", COLOR_ACTIVE);
            widgets::toggle(ui, &mut s.eq.enabled, "EQ", COLOR_ACTIVE);
            widgets::toggle(ui, &mut s.comp.enabled, "COMP", COLOR_ACTIVE);
            widgets::toggle(ui, &mut s.echo.enabled, "ECHO", COLOR_ACTIVE);
            widgets::toggle(ui, &mut s.reverb.enabled, "REVERB", COLOR_ACTIVE);
        });
    }

    fn fx_details(&mut self, ui: &mut Ui) {
        let s = &mut self.settings;
        let index = self.index;
        let meters = self.meters;
        egui::CollapsingHeader::new("FX settings").id_salt(("fx", index)).show(ui, |ui| {
            ui.spacing_mut().slider_width = widgets::DETAIL_SLIDER_WIDTH;
            ui.label(egui::RichText::new("Gate").strong());
            ui.add(egui::Slider::new(&mut s.gate.threshold_db, -80.0..=0.0).text("thresh dB"));
            ui.add(egui::Slider::new(&mut s.gate.release_ms, 10.0..=1000.0).text("release ms"));
            let open = meters.gate_open[index].load(std::sync::atomic::Ordering::Relaxed);
            ui.label(if s.gate.enabled && !open { "closed" } else { "open" });

            ui.label(egui::RichText::new("EQ").strong());
            widgets::toggle(ui, &mut s.eq.low_cut, "LOW CUT 80 Hz", COLOR_ACTIVE);
            ui.add(egui::Slider::new(&mut s.eq.bass_db, -12.0..=12.0).text("bass dB"));
            ui.add(egui::Slider::new(&mut s.eq.mid_db, -12.0..=12.0).text("mid dB"));
            ui.add(egui::Slider::new(&mut s.eq.treble_db, -12.0..=12.0).text("treble dB"));

            ui.label(egui::RichText::new("Compressor").strong());
            ui.add(egui::Slider::new(&mut s.comp.threshold_db, -60.0..=0.0).text("thresh dB"));
            ui.add(egui::Slider::new(&mut s.comp.ratio, 1.0..=20.0).text("ratio"));
            ui.add(egui::Slider::new(&mut s.comp.attack_ms, 0.1..=100.0).logarithmic(true).text("attack ms"));
            ui.add(egui::Slider::new(&mut s.comp.release_ms, 10.0..=1000.0).logarithmic(true).text("release ms"));
            ui.add(egui::Slider::new(&mut s.comp.makeup_db, 0.0..=24.0).text("makeup dB"));
            ui.label(format!("reduction {:.1} dB", meters.reduction(index)));

            ui.label(egui::RichText::new("Echo").strong());
            ui.add(egui::Slider::new(&mut s.echo.time_ms, 20.0..=2000.0).text("time ms"));
            ui.add(egui::Slider::new(&mut s.echo.feedback, 0.0..=0.95).text("feedback"));
            ui.add(egui::Slider::new(&mut s.echo.mix, 0.0..=1.0).text("mix"));

            ui.label(egui::RichText::new("Reverb").strong());
            ui.add(egui::Slider::new(&mut s.reverb.size, 0.0..=1.0).text("size"));
            ui.add(egui::Slider::new(&mut s.reverb.damping, 0.0..=1.0).text("damping"));
            ui.add(egui::Slider::new(&mut s.reverb.mix, 0.0..=1.0).text("mix"));
        });
    }
}

/// Two rows of bus buttons: A1..A5 then B1..B2, like Voicemeeter's routing grid.
fn routing_rows(ui: &mut Ui, routing: &mut [bool; NUM_BUSES], strip: usize) {
    let (hardware, virtual_buses) = routing.split_at_mut(NUM_HW_BUSES);
    ui.horizontal(|ui| {
        for (b, on) in hardware.iter_mut().enumerate() {
            ui.push_id((strip, b), |ui| widgets::toggle(ui, on, &bus_name(b), COLOR_ACTIVE));
        }
    });
    ui.horizontal(|ui| {
        for (offset, on) in virtual_buses.iter_mut().enumerate() {
            let b = NUM_HW_BUSES + offset;
            ui.push_id((strip, b), |ui| widgets::toggle(ui, on, &bus_name(b), COLOR_SOLO));
        }
    });
}
