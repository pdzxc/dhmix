//! Top-level eframe app: owns the audio I/O, the engine thread and the panels.

use super::bus_panel::BusView;
use super::player_panel::PlayerPanel;
use super::strip_panel::StripView;
use super::widgets::{self, PLAYER_WIDTH, STRIP_WIDTH};
use crate::audio::{list_devices, AudioIo, DeviceList};
use crate::engine::player::Player;
use crate::engine::runner::{self, EngineHandle, EngineInputs, RecordTap};
use crate::engine::{Meters, MixSettings};
use crate::hotkeys::{HotkeyAction, Hotkeys};
use crate::preset::Preset;
use crate::recorder::Recording;
use crate::{bus_name, strip_name, NUM_BUSES, NUM_STRIPS, PLAYER_STRIP};
use egui::Ui;
use parking_lot::Mutex;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

const STORAGE_KEY: &str = "streammix.preset";
const PRESET_FILTER: [&str; 1] = ["json"];

pub struct App {
    settings: Arc<Mutex<MixSettings>>,
    meters: Arc<Meters>,
    io: AudioIo,
    devices: DeviceList,
    player: PlayerPanel,
    record: Arc<RecordTap>,
    recording: Option<Recording>,
    record_bus: usize,
    hotkeys: Option<Hotkeys>,
    hotkey_strip: usize,
    hotkey_error: Option<String>,
    status: String,
    _engine: EngineHandle,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let preset: Preset = cc.storage.and_then(|s| eframe::get_value(s, STORAGE_KEY)).unwrap_or_default();

        let settings = Arc::new(Mutex::new(preset.mix));
        let meters = Arc::new(Meters::default());
        let record = Arc::new(RecordTap::default());
        let mut io = AudioIo::new();
        io.apply(&preset.io);
        let (player, player_tx, player_status) = Player::new();
        let engine = runner::spawn(EngineInputs {
            settings: settings.clone(),
            meters: meters.clone(),
            input_slots: io.input_slots(),
            output_slots: io.output_slots(),
            player,
            record: record.clone(),
        });

        let mut player_panel = PlayerPanel::new(player_tx, player_status);
        for (i, path) in preset.pads.iter().enumerate() {
            if let Some(p) = path {
                player_panel.assign_pad(i, p);
            }
        }
        if let Some(p) = &preset.music {
            player_panel.load_music(p, false);
        }

        let (hotkeys, hotkey_error) = match Hotkeys::register() {
            Ok(h) => (Some(h), None),
            Err(e) => (None, Some(format!("hotkeys unavailable: {e:#}"))),
        };

        Self {
            settings,
            meters,
            io,
            devices: list_devices(),
            player: player_panel,
            record,
            recording: None,
            record_bus: 0,
            hotkeys,
            hotkey_strip: preset.hotkey_strip.min(NUM_STRIPS - 1),
            hotkey_error,
            status: String::new(),
            _engine: engine,
        }
    }

    fn preset(&self) -> Preset {
        Preset {
            mix: *self.settings.lock(),
            io: self.io.settings.clone(),
            pads: self.player.pad_paths(),
            music: self.player.music_path(),
            hotkey_strip: self.hotkey_strip,
        }
    }

    fn apply_preset(&mut self, preset: Preset) {
        *self.settings.lock() = preset.mix;
        self.io.apply(&preset.io);
        for (i, path) in preset.pads.iter().enumerate() {
            match path {
                Some(p) => self.player.assign_pad(i, p),
                None => self.player.pads[i] = None,
            }
        }
        if let Some(p) = &preset.music {
            self.player.load_music(p, false);
        }
        self.hotkey_strip = preset.hotkey_strip.min(NUM_STRIPS - 1);
    }

    fn handle_hotkeys(&mut self) {
        let Some(hotkeys) = &self.hotkeys else { return };
        for action in hotkeys.poll() {
            match action {
                HotkeyAction::ToggleMute => {
                    let mut s = self.settings.lock();
                    let strip = &mut s.strips[self.hotkey_strip];
                    strip.mute = !strip.mute;
                }
                HotkeyAction::Pad(i) => self.player.trigger_pad(i),
            }
        }
    }

    fn toggle_recording(&mut self) {
        if let Some(rec) = self.recording.take() {
            *self.record.tx.lock() = None;
            self.status = match rec.stop() {
                Ok(()) => "Recording saved.".into(),
                Err(e) => format!("Recording failed: {e:#}"),
            };
            return;
        }
        let default_name = format!("streammix-{}.wav", chrono_like_stamp());
        let Some(path) = rfd::FileDialog::new().set_file_name(&default_name).add_filter("WAV", &["wav"]).save_file() else {
            return;
        };
        match Recording::start(&path) {
            Ok((rec, tx)) => {
                self.record.bus.store(self.record_bus, Ordering::Relaxed);
                *self.record.tx.lock() = Some(tx);
                self.recording = Some(rec);
                self.status = format!("Recording {} to {}", bus_name(self.record_bus), path.display());
            }
            Err(e) => self.status = format!("Could not start recording: {e:#}"),
        }
    }

    fn top_bar(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("StreamMix");
            ui.separator();
            if ui.button("Load preset…").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("Preset", &PRESET_FILTER).pick_file() {
                    match Preset::load(&path) {
                        Ok(p) => {
                            self.apply_preset(p);
                            self.status = format!("Loaded {}", path.display());
                        }
                        Err(e) => self.status = format!("Load failed: {e:#}"),
                    }
                }
            }
            if ui.button("Save preset…").clicked() {
                if let Some(path) = rfd::FileDialog::new().set_file_name("streammix.json").add_filter("Preset", &PRESET_FILTER).save_file() {
                    self.status = match self.preset().save(&path) {
                        Ok(()) => format!("Saved {}", path.display()),
                        Err(e) => format!("Save failed: {e:#}"),
                    };
                }
            }
            if ui.button("Refresh devices").clicked() {
                self.devices = list_devices();
            }
            ui.separator();
            ui.label("Record");
            egui::ComboBox::from_id_salt("record-bus").width(widgets::SMALL_COMBO_WIDTH).selected_text(bus_name(self.record_bus)).show_ui(ui, |ui| {
                for b in 0..NUM_BUSES {
                    ui.selectable_value(&mut self.record_bus, b, bus_name(b));
                }
            });
            let rec_label = if self.recording.is_some() { "■ Stop" } else { "● Rec" };
            if ui.button(rec_label).clicked() {
                self.toggle_recording();
            }
            ui.separator();
            ui.label("Mute hotkey strip");
            egui::ComboBox::from_id_salt("hotkey-strip").width(widgets::SMALL_COMBO_WIDTH).selected_text(strip_name(self.hotkey_strip)).show_ui(ui, |ui| {
                for i in 0..NUM_STRIPS {
                    ui.selectable_value(&mut self.hotkey_strip, i, strip_name(i));
                }
            });
            match &self.hotkey_error {
                Some(e) => widgets::error_label(ui, e),
                None => widgets::hint(ui, Hotkeys::description()),
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "48 kHz · 10 ms blocks · buffer {:.0} ms · input underruns {} · output drops {}",
                runner::ring_latency_ms(),
                self.meters.underruns.load(Ordering::Relaxed),
                self.meters.overruns.load(Ordering::Relaxed)
            ));
            if !self.status.is_empty() {
                ui.separator();
                ui.label(&self.status);
            }
            if let Some(err) = self.io.errors.last().cloned() {
                ui.separator();
                widgets::error_label(ui, &err);
                if ui.small_button("clear").clicked() {
                    self.io.errors.clear();
                }
            }
        });
    }

    fn strips_row(&mut self, ui: &mut Ui) {
        ui.horizontal_top(|ui| {
            for i in 0..NUM_STRIPS {
                let width = if i == PLAYER_STRIP { PLAYER_WIDTH } else { STRIP_WIDTH };
                ui.group(|ui| {
                    ui.set_width(width);
                    ui.vertical(|ui| {
                        let connected = self.meters.input_connected[i].load(Ordering::Relaxed);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(strip_name(i)).strong());
                            if !connected && i != PLAYER_STRIP {
                                widgets::hint(ui, "no device");
                            }
                        });
                        if i == PLAYER_STRIP {
                            self.player.show(ui);
                        } else {
                            let mut selected = self.io.settings.strip_inputs[i].clone();
                            if widgets::device_combo(ui, ("in", i), &mut selected, &self.devices.inputs) {
                                self.io.set_strip_input(i, selected);
                            }
                        }
                        let mut settings = self.settings.lock();
                        let any_solo = settings.any_solo();
                        StripView { index: i, settings: &mut settings.strips[i], meters: &self.meters, any_solo }.show(ui);
                    });
                });
            }
        });
    }

    fn buses_row(&mut self, ui: &mut Ui) {
        ui.horizontal_top(|ui| {
            for b in 0..NUM_BUSES {
                ui.group(|ui| {
                    ui.set_width(STRIP_WIDTH);
                    ui.vertical(|ui| {
                        let kind = if b < crate::NUM_HW_BUSES { "hardware out" } else { "virtual out" };
                        let connected = self.meters.output_connected[b].load(Ordering::Relaxed);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(bus_name(b)).strong());
                            widgets::hint(ui, if connected { kind } else { "no device" });
                        });
                        let mut selected = self.io.settings.bus_outputs[b].clone();
                        if widgets::device_combo(ui, ("out", b), &mut selected, &self.devices.outputs) {
                            self.io.set_bus_output(b, selected);
                        }
                        let mut settings = self.settings.lock();
                        BusView { index: b, settings: &mut settings.buses[b], meters: &self.meters }.show(ui);
                    });
                });
            }
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_hotkeys();
        egui::TopBottomPanel::top("top").show(ctx, |ui| self.top_bar(ui));
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                widgets::hint(ui, "INPUTS");
                self.strips_row(ui);
                ui.add_space(8.0);
                widgets::hint(ui, "OUTPUTS");
                self.buses_row(ui);
            });
        });
        ctx.request_repaint_after(Duration::from_millis(33));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, STORAGE_KEY, &self.preset());
    }
}

/// Seconds since the epoch, enough to make default recording names unique without another crate.
fn chrono_like_stamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
