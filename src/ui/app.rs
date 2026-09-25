//! Top-level eframe app: owns the audio I/O, the engine thread and the panels.

use super::apps_panel::{apps_feeding, apps_hearing, AppsPanel};
use crate::apps::{AppSession, Endpoint};
use std::time::Instant;
use super::bus_panel::BusView;
use super::help_panel::HelpPanel;
use super::player_panel::{PadBank, PlayerPanel};
use super::settings_panel::SettingsPanel;
use super::tray::{self, Tray};
use crate::app_settings::AppSettings;
use crate::startup;
use super::strip_panel::StripView;
use super::widgets::{self, ColumnFrame, Geometry, COLOR_MUTE};
use crate::audio::{list_devices, AudioIo, DeviceList};
use crate::engine::player::{Player, PlayerCommand};
use crate::engine::runner::{self, EngineHandle, EngineInputs, RecordTap};
use crate::engine::{Meters, MixSettings};
use crate::hotkeys::{HotkeyAction, Hotkeys};
use crate::preset::Preset;
use crate::recorder::Recording;
use crate::{bus_name, strip_name, NUM_BUSES, NUM_HW_BUSES, NUM_HW_STRIPS, NUM_STRIPS, PLAYER_STRIP};
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use egui::Ui;
use parking_lot::Mutex;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

const STORAGE_KEY: &str = "streammix.preset";
const APPS_REFRESH_EVERY: std::time::Duration = std::time::Duration::from_secs(3);
/// What a virtual input's or output's app list says when nothing is on its cable. The panel
/// layout tests render this exact wording, since it wraps in the fixed-width columns.
pub(super) const APPS_EMPTY_VIRTUAL_IN: &str = "No apps connected";
pub(super) const APPS_EMPTY_VIRTUAL_OUT: &str = "No apps connected";
const APPS_EMPTY_HARDWARE_IN: &str = "Physical";
const APPS_EMPTY_HARDWARE_OUT: &str = "Physical";
/// Set once the guide has been shown, so it only opens by itself on the very first launch.
const HELP_SEEN_KEY: &str = "streammix.help_seen";
const PRESET_FILTER: [&str; 1] = ["json"];
/// A comma-separated list of windows to open at launch (player, applications, help, settings),
/// for screenshots and testing: `DHMIX_OPEN=player,settings`.
const OPEN_WINDOWS_ENV: &str = "DHMIX_OPEN";

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

/// The bus group alone; the mixer header shows the bus name as a coloured tag after it.
fn bus_group_title(b: usize) -> &'static str {
    if b < NUM_HW_BUSES {
        "Hardware out"
    } else {
        "Virtual out"
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
    _hotkeys: Option<Hotkeys>,
    /// The strip Ctrl+Alt+M mutes; shared with the hotkey handler.
    hotkey_strip: Arc<AtomicUsize>,
    /// Whether Ctrl+Alt+1..9 fire pads; shared with the hotkey handler.
    pad_hotkeys: Arc<AtomicBool>,
    hotkey_error: Option<String>,
    app_settings: AppSettings,
    settings_panel: SettingsPanel,
    tray: Option<Tray>,
    /// Set by the tray's Quit, so the close that follows really quits.
    quit: Arc<AtomicBool>,
    status: String,
    apps: AppsPanel,
    /// Apps with audio sessions and the endpoints they can move to, refreshed on a timer.
    app_sessions: Vec<AppSession>,
    app_endpoints: Vec<Endpoint>,
    apps_refreshed: Option<Instant>,
    help: HelpPanel,
    /// The Player lives in its own floating window so inputs and outputs share one column grid.
    player_open: bool,
    fine_tune_open: [bool; NUM_STRIPS],
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

        let mut app_settings = AppSettings::load();
        app_settings.run_on_startup = startup::runs_on_startup();
        let hotkey_strip = Arc::new(AtomicUsize::new(preset.hotkey_strip.min(NUM_STRIPS - 1)));
        let pad_hotkeys = Arc::new(AtomicBool::new(app_settings.pad_hotkeys));
        let handler = hotkey_handler(settings.clone(), hotkey_strip.clone(), pad_hotkeys.clone(), player_panel.hotkey_target());
        let (hotkeys, hotkey_error) = match Hotkeys::register(handler) {
            Ok(h) => (Some(h), None),
            Err(e) => (None, Some(format!("Hotkeys unavailable: {e:#}"))),
        };
        let quit = Arc::new(AtomicBool::new(false));
        let tray = match Tray::new(cc.egui_ctx.clone(), quit.clone()) {
            Ok(tray) => Some(tray),
            Err(e) => {
                log::warn!("no tray icon: {e:#}");
                None
            }
        };
        let open_at_launch = windows_to_open(std::env::var(OPEN_WINDOWS_ENV).ok().as_deref());
        if !open_at_launch.is_empty() {
            log::info!("{OPEN_WINDOWS_ENV}: opening {open_at_launch:?} at launch");
        }
        if app_settings.start_in_tray && tray.is_none() {
            // Nothing could bring a hidden window back, so show it after all.
            tray::show_window(&cc.egui_ctx);
        }

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
            _hotkeys: hotkeys,
            hotkey_strip,
            pad_hotkeys,
            hotkey_error,
            app_settings,
            settings_panel: SettingsPanel { open: open_at_launch.contains(&"settings"), startup_error: None },
            tray,
            quit,
            status: String::new(),
            apps: AppsPanel::new(open_at_launch.contains(&"applications")),
            app_sessions: Vec::new(),
            app_endpoints: Vec::new(),
            apps_refreshed: None,
            help: HelpPanel { open: !help_seen || open_at_launch.contains(&"help") },
            player_open: open_at_launch.contains(&"player"),
            fine_tune_open: [false; NUM_STRIPS],
            _engine: engine,
        }
    }

    /// Re-reads which apps play or record, at most every few seconds. This runs whether or not
    /// the Applications window is open, because every virtual strip and bus lists its apps.
    fn refresh_apps_if_stale(&mut self) {
        if self.apps_refreshed.is_none_or(|t| t.elapsed() > APPS_REFRESH_EVERY) {
            (self.app_sessions, self.app_endpoints) = crate::apps::snapshot();
            self.apps_refreshed = Some(Instant::now());
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
            hotkey_strip: self.hotkey_strip.load(Ordering::Relaxed),
        }
    }

    fn apply_preset(&mut self, preset: Preset) {
        *self.settings.lock() = preset.mix;
        self.io.apply(&preset.io);
        for (i, path) in preset.pads.iter().enumerate() {
            match path {
                Some(p) => self.player.assign_pad(i, p),
                None => self.player.clear_pad(i),
            }
        }
        if let Some(p) = &preset.music {
            self.player.load_music(p, false);
        }
        self.hotkey_strip.store(preset.hotkey_strip.min(NUM_STRIPS - 1), Ordering::Relaxed);
    }

    /// Puts a changed setting into effect and saves it. "Run on startup" is asked of the OS; if
    /// it refuses, the toggle goes back and the window says why.
    fn apply_app_settings(&mut self) {
        self.pad_hotkeys.store(self.app_settings.pad_hotkeys, Ordering::Relaxed);
        let wanted = self.app_settings.run_on_startup;
        if wanted != startup::runs_on_startup() {
            match startup::set_run_on_startup(wanted) {
                Ok(()) => self.settings_panel.startup_error = None,
                Err(e) => {
                    self.app_settings.run_on_startup = !wanted;
                    self.settings_panel.startup_error = Some(format!("{e:#}"));
                }
            }
        }
        if let Err(e) = self.app_settings.save() {
            self.status = format!("Could not save settings: {e:#}");
        }
    }

    /// With "close to tray" on, the close button hides the window instead; the tray's Quit sets
    /// `quit` first so its close goes through.
    fn hide_instead_of_closing(&self, ctx: &egui::Context) {
        let closing = ctx.input(|i| i.viewport().close_requested());
        if closing && self.app_settings.close_to_tray && self.tray.is_some() && !self.quit.load(Ordering::Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
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
        let default_name = format!("dhmix-{}.wav", unix_stamp());
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

    /// The wordmark on the left; every control right-aligned. A right-to-left layout lays items
    /// out from the edge inwards, so each group is added in reverse reading order.
    fn top_bar(&mut self, ui: &mut Ui) {
        ui.add_space(widgets::SECTION_GAP);
        ui.horizontal(|ui| {
            widgets::logo(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.record_controls(ui);
                widgets::caption(ui, "RECORD");
                ui.separator();
                widgets::led(ui, &mut self.settings_panel.open, "SETTINGS", widgets::COLOR_ACCENT, widgets::TOP_TOGGLE_SIZE, "Tray, startup and hotkey options");
                widgets::led(ui, &mut self.help.open, "HELP", widgets::COLOR_ACCENT, widgets::TOP_TOGGLE_SIZE, "What inputs, outputs and the A / B buttons mean");
                widgets::led(ui, &mut self.apps.open, "APPLICATIONS", widgets::COLOR_ACCENT, widgets::TOP_TOGGLE_SIZE, "Which apps play or record audio, and where to send them");
                ui.separator();
                if widgets::button(ui, "SAVE…", widgets::TOP_BUTTON_SIZE, "Save the current mixer setup to a file") {
                    if let Some(path) = rfd::FileDialog::new().set_file_name("dhmix.json").add_filter("Preset", &PRESET_FILTER).save_file() {
                        self.status = match self.preset().save(&path) {
                            Ok(()) => format!("Saved {}", path.display()),
                            Err(e) => format!("Save failed: {e:#}"),
                        };
                    }
                }
                if widgets::button(ui, "LOAD…", widgets::TOP_BUTTON_SIZE, "Load a saved mixer setup") {
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
                widgets::caption(ui, "PRESET");
                ui.separator();
                if widgets::button(ui, "REFRESH", widgets::TOP_BUTTON_SIZE, "Rescan after plugging in a device or installing a cable") {
                    self.refresh_devices();
                }
                widgets::caption(ui, "DEVICES");
                ui.separator();
                widgets::primary_toggle(ui, &mut self.player_open, "PLAYER", widgets::COLOR_PLAYER, widgets::TOP_TOGGLE_SIZE, "Soundboard pads and music player, in its own window");
            });
        });
        // The status line: engine stats, the hotkeys, then whatever the last action reported.
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
            ui.separator();
            match &self.hotkey_error {
                Some(e) => widgets::error_label(ui, e),
                None => widgets::hint(ui, Hotkeys::description()),
            }
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

    /// The bus picker and the Record button, added button first because the top bar is laid out
    /// right to left.
    fn record_controls(&mut self, ui: &mut Ui) {
        if widgets::record_button(ui, self.recording.is_some()).clicked() {
            self.toggle_recording();
        }
        egui::ComboBox::from_id_salt("record-bus").width(widgets::SMALL_COMBO_WIDTH).selected_text(bus_name(self.record_bus)).show_ui(ui, |ui| {
            for b in 0..NUM_BUSES {
                ui.selectable_value(&mut self.record_bus, b, bus_name(b));
            }
        });
    }

    /// Draws the device inputs as one row of equal, fixed-size columns; the Player has its own
    /// window so this row has as many columns as the output row and lines up with it.
    fn strips_row(&mut self, ui: &mut Ui, row_height: f32) {
        let width = widgets::column_width(ui.available_width(), mixer_columns());
        let geo = Geometry::for_column(width);
        let fader_height = widgets::fader_height_in(row_height, widgets::INPUT_FIXED_HEIGHT);
        widgets::column_row(ui, |ui| {
            for i in 0..PLAYER_STRIP {
                if i == NUM_HW_STRIPS {
                    widgets::group_divider(ui, row_height);
                }
                let (apps, apps_empty) = if i < NUM_HW_STRIPS {
                    (Vec::new(), APPS_EMPTY_HARDWARE_IN)
                } else {
                    (apps_feeding(&self.app_sessions, self.io.settings.strip_inputs[i].as_deref()), APPS_EMPTY_VIRTUAL_IN)
                };
                widgets::panel(ui, width, row_height, |ui| {
                    let mut settings = self.settings.lock();
                    let any_solo = settings.any_solo();
                    let connected = self.meters.input_connected[i].load(Ordering::Relaxed);
                    let strip = &settings.strips[i];
                    let status = if strip.mute {
                        Some(("MUTED", COLOR_MUTE))
                    } else if any_solo && !strip.solo {
                        Some(("SILENT", widgets::COLOR_TEXT_MUTED))
                    } else if connected {
                        Some(("LIVE", widgets::COLOR_LIVE))
                    } else {
                        None
                    };
                    widgets::panel_header(ui, &strip_title(i), None, status);
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
                        apps: &apps,
                        apps_empty,
                    }
                    .show(ui);
                });
            }
        });
    }

    fn buses_row(&mut self, ui: &mut Ui, row_height: f32) {
        let width = widgets::column_width(ui.available_width(), mixer_columns());
        let geo = Geometry::for_column(width);
        let fader_height = widgets::fader_height_in(row_height, widgets::OUTPUT_FIXED_HEIGHT);
        widgets::column_row(ui, |ui| {
            for b in 0..NUM_BUSES {
                if b == NUM_HW_BUSES {
                    widgets::group_divider(ui, row_height);
                }
                let (apps, apps_empty) = if b < NUM_HW_BUSES {
                    (Vec::new(), APPS_EMPTY_HARDWARE_OUT)
                } else {
                    (apps_hearing(&self.app_sessions, self.io.settings.bus_outputs[b].as_deref()), APPS_EMPTY_VIRTUAL_OUT)
                };
                widgets::panel(ui, width, row_height, |ui| {
                    let mut settings = self.settings.lock();
                    let hardware = b < NUM_HW_BUSES;
                    let connected = self.meters.output_connected[b].load(Ordering::Relaxed);
                    let status = if settings.buses[b].mute {
                        Some(("MUTED", COLOR_MUTE))
                    } else if connected {
                        Some(("LIVE", widgets::COLOR_LIVE))
                    } else {
                        None
                    };
                    widgets::panel_header(ui, bus_group_title(b), Some((&bus_name(b), widgets::bus_color(b))), status);
                    let mut selected = self.io.settings.bus_outputs[b].clone();
                    let empty = if hardware { "Choose speakers / headphones…" } else { "Choose a cable input…" };
                    if widgets::device_combo(ui, ("out", b), &mut selected, &self.devices.outputs, empty, geo.inner) {
                        self.io.set_bus_output(b, selected);
                    }
                    BusView { index: b, settings: &mut settings.buses[b], meters: &self.meters, geo, fader_height, apps: &apps, apps_empty }.show(ui);
                });
            }
        });
    }
}

/// The window names in a `DHMIX_OPEN` value, trimmed and lower-cased.
fn windows_to_open(value: Option<&str>) -> Vec<&'static str> {
    const KNOWN: [&str; 4] = ["player", "applications", "help", "settings"];
    value
        .unwrap_or_default()
        .split(',')
        .filter_map(|name| KNOWN.iter().copied().find(|known| known.eq_ignore_ascii_case(name.trim())))
        .collect()
}

/// What a global hotkey does, on the OS event thread: mute the chosen strip, or fire a pad while
/// the pad hotkeys are enabled. `try_lock`, so a press during a modal file dialog (opened while
/// the UI holds a lock) is dropped rather than deadlocking.
fn hotkey_handler(
    settings: Arc<Mutex<MixSettings>>,
    strip: Arc<AtomicUsize>,
    pads_enabled: Arc<AtomicBool>,
    (bank, tx): (Arc<Mutex<PadBank>>, Sender<PlayerCommand>),
) -> impl Fn(HotkeyAction) + Send + Sync + 'static {
    move |action| match action {
        HotkeyAction::ToggleMute => {
            if let Some(mut s) = settings.try_lock() {
                let i = strip.load(Ordering::Relaxed).min(NUM_STRIPS - 1);
                s.strips[i].mute = !s.strips[i].mute;
            }
        }
        HotkeyAction::Pad(pad) => {
            if pads_enabled.load(Ordering::Relaxed) {
                if let Some(bank) = bank.try_lock() {
                    bank.trigger(&tx, pad);
                }
            }
        }
    }
}

/// Cable-like input names first, then the rest, so a virtual strip's likely pick is at the top.
fn virtual_first(devices: &DeviceList) -> Vec<String> {
    let cables = devices.virtual_inputs();
    let rest = devices.inputs.iter().filter(|n| !cables.contains(n)).cloned();
    cables.iter().cloned().chain(rest).collect()
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
        self.hide_instead_of_closing(ctx);
        self.refresh_apps_if_stale();
        egui::TopBottomPanel::top("top").show(ctx, |ui| self.top_bar(ui));
        egui::CentralPanel::default().frame(egui::Frame::default().fill(widgets::COLOR_BG).inner_margin(egui::Margin::same(widgets::SECTION_GAP))).show(ctx, |ui| {
            // Two captions, two rows: the rows share whatever height is left. Faders stop at
            // FADER_MIN_HEIGHT on their own, so a short window looks tight rather than pushing a
            // neighbouring column out of alignment.
            let captions = 2.0 * (widgets::SECTION_GAP + widgets::SECTION_CAPTION_HEIGHT) + widgets::SECTION_GAP;
            let rows_height = ui.available_height() - captions;
            let input_height = input_row_target(rows_height);
            widgets::section(ui, "Inputs");
            self.strips_row(ui, input_height);
            widgets::section(ui, "Outputs");
            let output_height = ui.available_height() - widgets::SECTION_GAP;
            self.buses_row(ui, output_height);
        });
        if self.apps.show(ctx, &self.io.settings, &self.app_sessions, &self.app_endpoints) {
            self.apps_refreshed = None;
        }
        self.player_window(ctx);
        self.help.show(ctx);
        let mute_strip = strip_name(self.hotkey_strip.load(Ordering::Relaxed));
        if self.settings_panel.show(ctx, &mut self.app_settings, self.tray.is_some(), &mute_strip) {
            self.apply_app_settings();
        }
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
        assert_eq!(bus_group_title(0), "Hardware out");
        assert_eq!(bus_group_title(NUM_HW_BUSES), "Virtual out");
    }

    #[test]
    fn inputs_and_outputs_share_one_column_grid() {
        assert_eq!(mixer_columns(), PLAYER_STRIP, "every device input sits above an output");
        assert_eq!(mixer_columns(), NUM_BUSES);
        assert_eq!(strip_title(PLAYER_STRIP), "Player", "the Player keeps its strip settings but lives in a window");
    }

    #[test]
    fn open_windows_env_names_only_known_windows() {
        assert_eq!(windows_to_open(Some("player, Settings,bogus")), vec!["player", "settings"]);
        assert!(windows_to_open(None).is_empty());
    }

    #[test]
    fn input_row_target_depends_only_on_the_available_viewport() {
        let rows_height = 700.0;
        assert_eq!(input_row_target(rows_height), 406.0);
        for _transient_panel_height in [406.0, 446.0, 405.0, 470.0] {
            assert_eq!(input_row_target(rows_height), 406.0, "transient panel measurements cannot affect the next frame");
        }
    }

    /// The hotkey handler is a plain closure over shared state: ToggleMute always flips the
    /// chosen strip's mute, and a pad only fires while `pad_hotkeys` is on, and only when a clip
    /// is actually loaded on it.
    #[test]
    fn hotkey_handler_toggles_mute_and_fires_pads_only_while_enabled() {
        use crate::ui::player_panel::Pad;
        use crate::engine::player::{Clip, PlayerCommand};

        let settings = Arc::new(Mutex::new(MixSettings::default()));
        let strip = Arc::new(AtomicUsize::new(2));
        let pads_enabled = Arc::new(AtomicBool::new(false));
        let samples = Arc::new(vec![0.1f32, -0.1]);
        let bank = Arc::new(Mutex::new(PadBank {
            pads: vec![Some(Pad { path: std::path::PathBuf::from("clip.wav"), clip: Clip { name: "clip".into(), samples: samples.clone() } }), None],
        }));
        let (tx, rx) = crossbeam_channel::unbounded();
        let handler = hotkey_handler(settings.clone(), strip.clone(), pads_enabled.clone(), (bank.clone(), tx));

        assert!(!settings.lock().strips[2].mute);
        handler(HotkeyAction::ToggleMute);
        assert!(settings.lock().strips[2].mute, "ToggleMute must flip the chosen strip's mute");
        handler(HotkeyAction::ToggleMute);
        assert!(!settings.lock().strips[2].mute, "and flip it back");

        // Pads are disabled by default: a loaded pad must not fire.
        handler(HotkeyAction::Pad(0));
        assert!(rx.try_recv().is_err(), "a pad hotkey must do nothing while pad_hotkeys is off");

        pads_enabled.store(true, Ordering::Relaxed);
        handler(HotkeyAction::Pad(0));
        match rx.try_recv().expect("a loaded pad must fire once pad_hotkeys is on") {
            PlayerCommand::PlayPad { pad, samples: sent } => {
                assert_eq!(pad, 0);
                assert!(Arc::ptr_eq(&sent, &samples));
            }
            _ => panic!("expected PlayPad"),
        }

        // An empty pad slot fires nothing even while enabled.
        handler(HotkeyAction::Pad(1));
        assert!(rx.try_recv().is_err(), "an empty pad must send nothing even while pad_hotkeys is on");
    }
}
