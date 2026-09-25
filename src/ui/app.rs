//! Top-level eframe app: owns the audio I/O, the engine thread and the panels.

use super::apps_panel::AppsPanel;
use super::bus_panel::BusView;
use super::help_panel::HelpPanel;
use super::player_panel::PlayerPanel;
use super::strip_panel::StripView;
use super::widgets::{self, ColumnFrame, Geometry, COLOR_ACTIVE, COLOR_MUTE, COLOR_VIRTUAL};
use crate::audio::{list_devices, AudioIo, DeviceList};
use crate::engine::player::Player;
use crate::engine::runner::{self, EngineHandle, EngineInputs, RecordTap};
use crate::engine::{Meters, MixSettings};
use crate::hotkeys::{HotkeyAction, Hotkeys};
use crate::preset::Preset;
use crate::recorder::Recording;
use crate::{bus_name, strip_name, NUM_BUSES, NUM_HW_BUSES, NUM_HW_STRIPS, NUM_STRIPS, PLAYER_STRIP};
use egui::Ui;
use parking_lot::Mutex;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

const STORAGE_KEY: &str = "streammix.preset";
/// Set once the guide has been shown, so it only opens by itself on the very first launch.
const HELP_SEEN_KEY: &str = "streammix.help_seen";
const PRESET_FILTER: [&str; 1] = ["json"];
const SETUP_STEPS: usize = 3;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Workspace {
    #[default]
    Mixer,
    Devices,
}

impl Workspace {
    const NAVIGATION: [(Self, &'static str); 2] = [(Self::Mixer, "Mixer"), (Self::Devices, "Devices")];
}

#[derive(Clone, Copy)]
enum DeviceRoute {
    HardwareInput(usize),
    VirtualInput(usize),
    HardwareOutput(usize),
    VirtualOutput(usize),
}

impl DeviceRoute {
    fn is_input(self) -> bool {
        matches!(self, Self::HardwareInput(_) | Self::VirtualInput(_))
    }

    fn is_virtual(self) -> bool {
        matches!(self, Self::VirtualInput(_) | Self::VirtualOutput(_))
    }

    fn index(self) -> usize {
        match self {
            Self::HardwareInput(index) | Self::VirtualInput(index) | Self::HardwareOutput(index) | Self::VirtualOutput(index) => index,
        }
    }
}

struct DeviceCard<'a> {
    title: String,
    purpose: &'a str,
    assigned: Option<String>,
    connected: bool,
    choices: Vec<String>,
    empty: &'a str,
    route: DeviceRoute,
    cable_note: Option<&'a str>,
}

/// Console-style panel titles.
fn strip_title(i: usize) -> String {
    if i < NUM_HW_STRIPS {
        format!("Hardware input {}", i + 1)
    } else if i < PLAYER_STRIP {
        format!("Virtual input {}", i - NUM_HW_STRIPS + 1)
    } else {
        "Player".to_string()
    }
}

fn bus_title(b: usize) -> String {
    if b < NUM_HW_BUSES {
        format!("Hardware out {}", bus_name(b))
    } else {
        format!("Virtual out {}", bus_name(b))
    }
}

/// Inputs and outputs share one grid: the device strips above, the buses below, same count.
const _: () = assert!(PLAYER_STRIP == NUM_BUSES, "device inputs and outputs must share a column count");
/// The Player window must be at least a column wide, or Geometry's floor would exceed the window.
const _: () = assert!(widgets::PLAYER_WINDOW_WIDTH + 2.0 * widgets::PANEL_PADDING >= widgets::MIN_COLUMN_WIDTH);

fn mixer_columns() -> usize {
    NUM_BUSES
}

fn input_row_target(rows_height: f32) -> f32 {
    (rows_height * widgets::INPUT_ROW_SHARE).floor()
}

pub struct App {
    settings: Arc<Mutex<MixSettings>>,
    meters: Arc<Meters>,
    io: AudioIo,
    devices: DeviceList,
    /// Input names with cable endpoints first, for the virtual strips.
    virtual_first_inputs: Vec<String>,
    player: PlayerPanel,
    record: Arc<RecordTap>,
    recording: Option<Recording>,
    record_bus: usize,
    hotkeys: Option<Hotkeys>,
    hotkey_strip: usize,
    hotkey_error: Option<String>,
    status: String,
    apps: AppsPanel,
    help: HelpPanel,
    /// The Player lives in its own floating window so inputs and outputs share one column grid.
    player_open: bool,
    fine_tune_open: [bool; NUM_STRIPS],
    /// Per-column fader heights, nudged every frame so each panel fills its row.
    strip_fader_height: [f32; NUM_STRIPS],
    /// Tallest input panel last frame; every input stretches to it so the row is level.
    input_row_height: f32,
    bus_fader_height: [f32; NUM_BUSES],
    workspace: Workspace,
    _engine: EngineHandle,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        widgets::apply_theme(&cc.egui_ctx);
        let preset: Preset = cc.storage.and_then(|s| eframe::get_value(s, STORAGE_KEY)).unwrap_or_default();
        let help_seen: bool = cc.storage.and_then(|s| eframe::get_value(s, HELP_SEEN_KEY)).unwrap_or(false);

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
            Err(e) => (None, Some(format!("Hotkeys unavailable: {e:#}"))),
        };

        let devices = list_devices();
        Self {
            settings,
            meters,
            io,
            virtual_first_inputs: virtual_first(&devices),
            devices,
            player: player_panel,
            record,
            recording: None,
            record_bus: 0,
            hotkeys,
            hotkey_strip: preset.hotkey_strip.min(NUM_STRIPS - 1),
            hotkey_error,
            status: String::new(),
            apps: AppsPanel::default(),
            help: HelpPanel { open: !help_seen },
            player_open: false,
            fine_tune_open: [false; NUM_STRIPS],
            strip_fader_height: [widgets::FADER_MIN_HEIGHT; NUM_STRIPS],
            input_row_height: 0.0,
            bus_fader_height: [widgets::FADER_MIN_HEIGHT; NUM_BUSES],
            workspace: Workspace::Mixer,
            _engine: engine,
        }
    }

    fn refresh_devices(&mut self) {
        self.devices = list_devices();
        self.virtual_first_inputs = virtual_first(&self.devices);
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
        let default_name = format!("streammix-{}.wav", unix_stamp());
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

    fn setup_progress(&self) -> usize {
        setup_progress(&self.io.settings.strip_inputs, &self.io.settings.bus_outputs)
    }

    fn setup_incomplete(&self) -> bool {
        self.setup_progress() < SETUP_STEPS
    }

    fn top_bar(&mut self, ui: &mut Ui) {
        ui.add_space(widgets::SECTION_GAP);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("STREAMMIX").heading().color(widgets::COLOR_HEADING));
            widgets::hint(ui, "mixer for streaming");
            ui.separator();
            for (workspace, label) in Workspace::NAVIGATION {
                ui.selectable_value(&mut self.workspace, workspace, label);
            }
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
            if ui.button("Refresh devices").on_hover_text("Rescan after plugging in a device or installing a cable").clicked() {
                self.refresh_devices();
            }
            let apps_label = if self.apps.open { "Applications ▾" } else { "Applications" };
            if ui.button(apps_label).on_hover_text("See which apps play or record audio and move them between devices").clicked() {
                self.apps.open = !self.apps.open;
            }
            let player_label = if self.player_open { "Player ▾" } else { "Player" };
            if ui.button(player_label).on_hover_text("Soundboard and music player, in its own window").clicked() {
                self.player_open = !self.player_open;
            }
            if ui.button("Help").on_hover_text("What inputs, outputs and the A / B buttons mean").clicked() {
                self.help.open = !self.help.open;
            }
            ui.separator();
            self.record_controls(ui);
            ui.separator();
            widgets::hint(ui, "Mute hotkey");
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
            widgets::hint(
                ui,
                format!(
                    "48 kHz · 10 ms blocks · {:.0} ms buffer · input underruns {} · output drops {}",
                    runner::ring_latency_ms(),
                    self.meters.underruns.load(Ordering::Relaxed),
                    self.meters.overruns.load(Ordering::Relaxed)
                ),
            );
            if !self.status.is_empty() {
                ui.separator();
                ui.label(&self.status);
            }
            if let Some(err) = self.io.errors.last().cloned() {
                ui.separator();
                widgets::error_label(ui, &err);
                if ui.small_button("Dismiss").clicked() {
                    self.io.errors.clear();
                }
            }
        });
        ui.add_space(widgets::SECTION_GAP);
    }

    fn record_controls(&mut self, ui: &mut Ui) {
        widgets::hint(ui, "Record");
        egui::ComboBox::from_id_salt("record-bus").width(widgets::SMALL_COMBO_WIDTH).selected_text(bus_name(self.record_bus)).show_ui(ui, |ui| {
            for b in 0..NUM_BUSES {
                ui.selectable_value(&mut self.record_bus, b, bus_name(b));
            }
        });
        let clicked = if self.recording.is_some() {
            widgets::action_button(ui, "■ Stop recording", COLOR_MUTE).clicked()
        } else {
            ui.button("● Record").on_hover_text("Save the chosen bus as a WAV file").clicked()
        };
        if clicked {
            self.toggle_recording();
        }
    }

    /// First-run guidance stays visible until the essential mic, listening, and stream routes exist.
    fn setup_banner(&mut self, ui: &mut Ui) {
        let progress = self.setup_progress();
        widgets::banner(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(format!("Finish setup — {progress}/{SETUP_STEPS} connected")).strong());
                ui.label("Choose a microphone, a listening device, and a virtual cable for your stream or call.");
                if ui.button("Continue device setup").clicked() {
                    self.workspace = Workspace::Devices;
                }
            });
        });
        ui.add_space(widgets::SECTION_GAP);
    }

    fn device_card(&mut self, ui: &mut Ui, card: DeviceCard<'_>) {
        let status = if card.connected {
            Some(("LIVE", if card.route.is_virtual() { COLOR_VIRTUAL } else { COLOR_ACTIVE }))
        } else if card.assigned.is_some() {
            Some(("ASSIGNED", if card.route.is_virtual() { COLOR_VIRTUAL } else { COLOR_ACTIVE }))
        } else {
            Some(("UNASSIGNED", COLOR_MUTE))
        };
        let width = ui.available_width();
        widgets::panel(ui, width, |ui| {
            widgets::panel_header(ui, &card.title, status);
            widgets::hint(ui, card.purpose);
            if let Some(cable) = card.cable_note {
                widgets::hint(ui, cable);
            }
            ui.add_space(widgets::ITEM_SPACING);
            let mut selected = card.assigned;
            if widgets::device_combo(ui, ("device-workspace", card.route.is_input(), card.route.index()), &mut selected, &card.choices, card.empty, ui.available_width()) {
                if card.route.is_input() {
                    self.io.set_strip_input(card.route.index(), selected);
                } else {
                    self.io.set_bus_output(card.route.index(), selected);
                }
            }
        });
    }

    fn devices_view(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            widgets::banner(ui, |ui| {
                ui.label(egui::RichText::new(format!("Device setup: {}/{} essentials connected", self.setup_progress(), SETUP_STEPS)).strong());
                widgets::hint(ui, "Assign sources first, then choose where each mix leaves Streammix. Changes take effect immediately.");
            });

            widgets::section(ui, "Finish setup");
            self.device_card(ui, DeviceCard {
                title: strip_title(0),
                purpose: "1. Primary microphone or main physical source.",
                assigned: self.io.settings.strip_inputs[0].clone(),
                connected: self.meters.input_connected[0].load(Ordering::Relaxed),
                choices: self.devices.inputs.clone(),
                empty: "Choose a microphone or input…",
                route: DeviceRoute::HardwareInput(0),
                cable_note: None,
            });
            ui.add_space(widgets::SECTION_GAP);
            self.device_card(ui, DeviceCard {
                title: bus_title(0),
                purpose: "2. Your main headphones or speakers.",
                assigned: self.io.settings.bus_outputs[0].clone(),
                connected: self.meters.output_connected[0].load(Ordering::Relaxed),
                choices: self.devices.outputs.clone(),
                empty: "Choose speakers or headphones…",
                route: DeviceRoute::HardwareOutput(0),
                cable_note: None,
            });
            ui.add_space(widgets::SECTION_GAP);
            self.device_card(ui, DeviceCard {
                title: bus_title(NUM_HW_BUSES),
                purpose: "3. Send the mix to a streaming or calling app through a virtual cable.",
                assigned: self.io.settings.bus_outputs[NUM_HW_BUSES].clone(),
                connected: self.meters.output_connected[NUM_HW_BUSES].load(Ordering::Relaxed),
                choices: self.devices.outputs.clone(),
                empty: "Choose a cable input…",
                route: DeviceRoute::VirtualOutput(NUM_HW_BUSES),
                cable_note: Some("Cable direction: select the cable’s Input here; choose that cable as the app’s microphone or source."),
            });

            widgets::section(ui, "Optional hardware inputs");
            for i in 1..NUM_HW_STRIPS {
                self.device_card(ui, DeviceCard {
                    title: strip_title(i),
                    purpose: "Optional microphone, instrument, or capture source.",
                    assigned: self.io.settings.strip_inputs[i].clone(),
                    connected: self.meters.input_connected[i].load(Ordering::Relaxed),
                    choices: self.devices.inputs.clone(),
                    empty: "Choose a microphone or input…",
                    route: DeviceRoute::HardwareInput(i),
                    cable_note: None,
                });
                ui.add_space(widgets::SECTION_GAP);
            }

            widgets::section(ui, "Virtual inputs");
            for i in NUM_HW_STRIPS..PLAYER_STRIP {
                self.device_card(ui, DeviceCard {
                    title: strip_title(i),
                    purpose: "Bring audio from an app or virtual cable into the mixer.",
                    assigned: self.io.settings.strip_inputs[i].clone(),
                    connected: self.meters.input_connected[i].load(Ordering::Relaxed),
                    choices: self.virtual_first_inputs.clone(),
                    empty: "Choose a cable output…",
                    route: DeviceRoute::VirtualInput(i),
                    cable_note: Some("Cable direction: select the cable’s Output here; the app sends audio into that cable."),
                });
                ui.add_space(widgets::SECTION_GAP);
            }

            widgets::section(ui, "Optional hardware outputs");
            for b in 1..NUM_HW_BUSES {
                self.device_card(ui, DeviceCard {
                    title: bus_title(b),
                    purpose: "Optional monitor, speaker, or recording destination.",
                    assigned: self.io.settings.bus_outputs[b].clone(),
                    connected: self.meters.output_connected[b].load(Ordering::Relaxed),
                    choices: self.devices.outputs.clone(),
                    empty: "Choose speakers or headphones…",
                    route: DeviceRoute::HardwareOutput(b),
                    cable_note: None,
                });
                ui.add_space(widgets::SECTION_GAP);
            }

            widgets::section(ui, "Optional virtual outputs");
            for b in NUM_HW_BUSES + 1..NUM_BUSES {
                self.device_card(ui, DeviceCard {
                    title: bus_title(b),
                    purpose: "Send this mix to a streaming, calling, or recording app through a virtual cable.",
                    assigned: self.io.settings.bus_outputs[b].clone(),
                    connected: self.meters.output_connected[b].load(Ordering::Relaxed),
                    choices: self.devices.outputs.clone(),
                    empty: "Choose a cable input…",
                    route: DeviceRoute::VirtualOutput(b),
                    cable_note: Some("Cable direction: select the cable’s Input here; choose that cable as the app’s microphone or source."),
                });
                ui.add_space(widgets::SECTION_GAP);
            }
        });
    }

    /// Draws the device inputs as one row of equal columns; the Player has its own window so this
    /// row has as many columns as the output row and lines up with it. Returns the tallest panel
    /// so the next frame can level the row to it.
    fn strips_row(&mut self, ui: &mut Ui, row_height: f32) -> f32 {
        let width = widgets::column_width(ui.available_width(), mixer_columns());
        let geo = Geometry::for_column(width);
        let mut tallest: f32 = 0.0;
        ui.horizontal_top(|ui| {
            for i in 0..PLAYER_STRIP {
                let fader_height = self.strip_fader_height[i];
                let rect = widgets::panel(ui, width, |ui| {
                    let mut settings = self.settings.lock();
                    let any_solo = settings.any_solo();
                    let connected = self.meters.input_connected[i].load(Ordering::Relaxed);
                    let status = if settings.strips[i].mute {
                        Some(("MUTED", COLOR_MUTE))
                    } else if connected {
                        Some(("LIVE", COLOR_ACTIVE))
                    } else {
                        None
                    };
                    widgets::panel_header(ui, &strip_title(i), status);
                    let mut selected = self.io.settings.strip_inputs[i].clone();
                    let (names, empty) = if i < NUM_HW_STRIPS {
                        (&self.devices.inputs, "Choose a microphone…")
                    } else {
                        (&self.virtual_first_inputs, "Choose a cable output…")
                    };
                    if widgets::device_combo(ui, ("in", i), &mut selected, names, empty, geo.inner) {
                        self.io.set_strip_input(i, selected);
                    }
                    StripView {
                        index: i,
                        settings: &mut settings.strips[i],
                        meters: &self.meters,
                        any_solo,
                        fine_tune_open: &mut self.fine_tune_open[i],
                        geo,
                        fader_height,
                    }
                    .show(ui);
                });
                tallest = tallest.max(rect.height());
                self.strip_fader_height[i] = widgets::stretch_towards(fader_height, rect.height(), row_height);
            }
        });
        tallest
    }

    fn buses_row(&mut self, ui: &mut Ui, row_height: f32) {
        let width = widgets::column_width(ui.available_width(), mixer_columns());
        let geo = Geometry::for_column(width);
        ui.horizontal_top(|ui| {
            for b in 0..NUM_BUSES {
                let fader_height = self.bus_fader_height[b];
                let rect = widgets::panel(ui, width, |ui| {
                    let mut settings = self.settings.lock();
                    let hardware = b < NUM_HW_BUSES;
                    let connected = self.meters.output_connected[b].load(Ordering::Relaxed);
                    let status = if settings.buses[b].mute {
                        Some(("MUTED", COLOR_MUTE))
                    } else if connected {
                        Some(("LIVE", if hardware { COLOR_ACTIVE } else { COLOR_VIRTUAL }))
                    } else {
                        None
                    };
                    widgets::panel_header(ui, &bus_title(b), status);
                    let mut selected = self.io.settings.bus_outputs[b].clone();
                    let empty = if hardware { "Choose speakers / headphones…" } else { "Choose a cable input…" };
                    if widgets::device_combo(ui, ("out", b), &mut selected, &self.devices.outputs, empty, geo.inner) {
                        self.io.set_bus_output(b, selected);
                    }
                    BusView { index: b, settings: &mut settings.buses[b], meters: &self.meters, geo, fader_height }.show(ui);
                });
                self.bus_fader_height[b] = widgets::stretch_towards(fader_height, rect.height(), row_height);
            }
        });
    }
}

/// Cable-like input names first, then the rest, so a virtual strip's likely pick is at the top.
fn virtual_first(devices: &DeviceList) -> Vec<String> {
    let cables = devices.virtual_inputs();
    let rest = devices.inputs.iter().filter(|n| !cables.contains(n)).cloned();
    cables.iter().cloned().chain(rest).collect()
}

/// Essential first-run routes: source, listening device, and virtual app destination.
fn setup_progress(strip_inputs: &[Option<String>], bus_outputs: &[Option<String>]) -> usize {
    [
        strip_inputs.first().is_some_and(Option::is_some),
        bus_outputs.first().is_some_and(Option::is_some),
        bus_outputs.get(NUM_HW_BUSES).is_some_and(Option::is_some),
    ]
    .into_iter()
    .filter(|connected| *connected)
    .count()
}

impl App {
    /// The soundboard and music player, in a floating window with the same controls as a strip.
    fn player_window(&mut self, ctx: &egui::Context) {
        if !self.player_open {
            return;
        }
        let mut open = self.player_open;
        egui::Window::new("Player").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            let width = widgets::PLAYER_WINDOW_WIDTH;
            ui.set_width(width);
            let mut settings = self.settings.lock();
            let any_solo = settings.any_solo();
            if settings.strips[PLAYER_STRIP].mute {
                widgets::hint(ui, "Muted on every bus");
            }
            let frame = ColumnFrame { geo: Geometry::for_column(width + 2.0 * widgets::PANEL_PADDING), fader_height: widgets::PLAYER_WINDOW_FADER_HEIGHT, any_solo };
            self.player.show(ui, &mut settings.strips[PLAYER_STRIP], &self.meters, &mut self.fine_tune_open[PLAYER_STRIP], frame);
        });
        self.player_open = open;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_hotkeys();
        egui::TopBottomPanel::top("top").show(ctx, |ui| self.top_bar(ui));
        egui::CentralPanel::default().frame(egui::Frame::default().fill(widgets::COLOR_BG).inner_margin(egui::Margin::same(widgets::SECTION_GAP))).show(ctx, |ui| {
            match self.workspace {
                Workspace::Devices => self.devices_view(ui),
                Workspace::Mixer => {
                    if self.setup_incomplete() {
                        self.setup_banner(ui);
                    }
                    // Two captions, two rows: the rows share whatever height is left. Faders stop
                    // at FADER_MIN_HEIGHT on their own, so a short window looks tight rather than
                    // pushing a neighbouring column out of alignment.
                    let captions = 2.0 * (widgets::SECTION_GAP + widgets::SECTION_CAPTION_HEIGHT) + widgets::SECTION_GAP;
                    let rows_height = ui.available_height() - captions;
                    // Inputs get their share, or more if one input panel (the Player) needs it.
                    let input_height = input_row_target(rows_height).max(self.input_row_height);
                    widgets::section(ui, "Inputs");
                    self.input_row_height = self.strips_row(ui, input_height);
                    widgets::section(ui, "Outputs");
                    let output_height = ui.available_height() - widgets::SECTION_GAP;
                    self.buses_row(ui, output_height);
                }
            }
        });
        self.apps.show(ctx, &self.io.settings);
        self.player_window(ctx);
        self.help.show(ctx);
        ctx.request_repaint_after(Duration::from_millis(33));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, STORAGE_KEY, &self.preset());
        eframe::set_value(storage, HELP_SEEN_KEY, &true);
    }
}

/// Seconds since the epoch, enough to make default recording names unique without another crate.
fn unix_stamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_strips_list_cable_endpoints_before_other_inputs() {
        let devices = DeviceList {
            inputs: vec!["Realtek Mic".into(), "CABLE Output (VB-Audio)".into(), "Webcam Mic".into()],
            outputs: vec![],
        };
        assert_eq!(virtual_first(&devices), vec!["CABLE Output (VB-Audio)".to_string(), "Realtek Mic".into(), "Webcam Mic".into()]);
    }

    #[test]
    fn panel_titles_read_like_the_console() {
        assert_eq!(strip_title(0), "Hardware input 1");
        assert_eq!(strip_title(NUM_HW_STRIPS), "Virtual input 1");
        assert_eq!(strip_title(PLAYER_STRIP), "Player");
        assert_eq!(bus_title(0), "Hardware out A1");
        assert_eq!(bus_title(NUM_HW_BUSES), "Virtual out B1");
    }

    #[test]
    fn setup_progress_tracks_all_three_essential_routes() {
        let mut inputs = vec![None; NUM_STRIPS];
        let mut outputs = vec![None; NUM_BUSES];
        assert_eq!(setup_progress(&inputs, &outputs), 0);

        inputs[0] = Some("Microphone".into());
        outputs[0] = Some("Headphones".into());
        assert_eq!(setup_progress(&inputs, &outputs), 2);

        outputs[NUM_HW_BUSES] = Some("CABLE Input".into());
        assert_eq!(setup_progress(&inputs, &outputs), SETUP_STEPS);
    }

    #[test]
    fn device_routes_preserve_endpoint_direction_and_type() {
        let hardware_input = DeviceRoute::HardwareInput(1);
        let virtual_output = DeviceRoute::VirtualOutput(NUM_HW_BUSES);

        assert!(hardware_input.is_input());
        assert!(!hardware_input.is_virtual());
        assert_eq!(hardware_input.index(), 1);
        assert!(!virtual_output.is_input());
        assert!(virtual_output.is_virtual());
        assert_eq!(virtual_output.index(), NUM_HW_BUSES);
    }

    #[test]
    fn workspace_navigation_is_mixer_then_devices() {
        assert_eq!(Workspace::NAVIGATION, [(Workspace::Mixer, "Mixer"), (Workspace::Devices, "Devices")]);
    }

    #[test]
    fn device_inputs_and_outputs_share_one_column_grid() {
        assert_eq!(mixer_columns(), PLAYER_STRIP, "every device input sits above an output");
        assert_eq!(mixer_columns(), NUM_BUSES);
        assert_eq!(strip_title(PLAYER_STRIP), "Player", "the Player keeps its strip settings but lives in a window");
    }

    #[test]
    fn input_row_target_depends_only_on_the_available_viewport() {
        let rows_height = 700.0;
        assert_eq!(input_row_target(rows_height), 406.0);
        for _transient_panel_height in [406.0, 446.0, 405.0, 470.0] {
            assert_eq!(input_row_target(rows_height), 406.0, "transient panel measurements cannot affect the next frame");
        }
    }
}
