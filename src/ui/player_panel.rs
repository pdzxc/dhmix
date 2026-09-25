//! Soundboard pads and the music player, shown at the top of the PLAYER strip.

use super::strip_panel::{fine_tune_window, fx_row, routing_rows, state_column};
use super::widgets::{self, ColumnFrame, Geometry, COLOR_ACTIVE, COLOR_ASSIGNED, PAD_SPACING};
use crate::engine::player::{load_clip, Clip, PlayerCommand, PlayerStatus};
use crate::engine::{Meters, StripSettings};
use crate::{db_to_gain, PLAYER_STRIP};
use crate::preset::NUM_PADS;
use crate::SAMPLE_RATE;
use crossbeam_channel::Sender;
use egui::Ui;
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

const PAD_COLUMNS: usize = 3;
const AUDIO_EXTENSIONS: [&str; 7] = ["mp3", "wav", "flac", "ogg", "m4a", "aac", "mp4"];
const PAD_TIPS: [&str; NUM_PADS] = [
    "Play (Ctrl+Alt+1). Right-click to change.",
    "Play (Ctrl+Alt+2). Right-click to change.",
    "Play (Ctrl+Alt+3). Right-click to change.",
    "Play (Ctrl+Alt+4). Right-click to change.",
    "Play (Ctrl+Alt+5). Right-click to change.",
    "Play (Ctrl+Alt+6). Right-click to change.",
    "Play (Ctrl+Alt+7). Right-click to change.",
    "Play (Ctrl+Alt+8). Right-click to change.",
    "Play (Ctrl+Alt+9). Right-click to change.",
];
const EMPTY_PAD_TIP: &str = "Click to choose a sound file.";

pub struct Pad {
    pub path: PathBuf,
    pub clip: Clip,
}

/// The loaded pads and their volume, shared with the hotkey handler so Ctrl+Alt+1..9 can fire
/// a pad from the OS event thread while the window is hidden.
pub struct PadBank {
    pub pads: Vec<Option<Pad>>,
}

impl PadBank {
    /// Plays `pad` if a clip is loaded on it.
    pub fn trigger(&self, tx: &Sender<PlayerCommand>, pad: usize) {
        if let Some(Some(p)) = self.pads.get(pad) {
            let _ = tx.send(PlayerCommand::PlayPad { pad, samples: p.clip.samples.clone() });
        }
    }
}

pub struct PlayerPanel {
    pub bank: Arc<Mutex<PadBank>>,
    pub music: Option<(PathBuf, Clip)>,
    pub music_loop: bool,
    /// The music's and the pads' own faders, in dB, ahead of the PLAYER strip's fader.
    pub music_db: f32,
    pub pads_db: f32,
    pub last_error: Option<String>,
    tx: Sender<PlayerCommand>,
    status: std::sync::Arc<PlayerStatus>,
    seek_target: Option<f32>,
}

impl PlayerPanel {
    pub fn new(tx: Sender<PlayerCommand>, status: std::sync::Arc<PlayerStatus>) -> Self {
        Self {
            bank: Arc::new(Mutex::new(PadBank { pads: (0..NUM_PADS).map(|_| None).collect() })),
            music: None,
            music_loop: false,
            music_db: 0.0,
            pads_db: 0.0,
            last_error: None,
            tx,
            status,
            seek_target: None,
        }
    }

    pub fn pad_paths(&self) -> Vec<Option<PathBuf>> {
        self.bank.lock().pads.iter().map(|p| p.as_ref().map(|p| p.path.clone())).collect()
    }

    /// The handles a hotkey handler needs to fire pads without the panel.
    pub fn hotkey_target(&self) -> (Arc<Mutex<PadBank>>, Sender<PlayerCommand>) {
        (self.bank.clone(), self.tx.clone())
    }

    pub fn clear_pad(&self, pad: usize) {
        self.bank.lock().pads[pad] = None;
    }

    pub fn music_path(&self) -> Option<PathBuf> {
        self.music.as_ref().map(|(p, _)| p.clone())
    }

    pub fn assign_pad(&mut self, pad: usize, path: &Path) {
        match load_clip(path) {
            Ok(clip) => self.bank.lock().pads[pad] = Some(Pad { path: path.to_path_buf(), clip }),
            Err(e) => self.last_error = Some(format!("{}: {e:#}", path.display())),
        }
    }

    pub fn load_music(&mut self, path: &Path, autoplay: bool) {
        match load_clip(path) {
            Ok(clip) => {
                let _ = self.tx.send(PlayerCommand::LoadMusic { samples: clip.samples.clone() });
                if !autoplay {
                    let _ = self.tx.send(PlayerCommand::MusicStop);
                }
                self.music = Some((path.to_path_buf(), clip));
            }
            Err(e) => self.last_error = Some(format!("{}: {e:#}", path.display())),
        }
    }

    pub fn trigger_pad(&self, pad: usize) {
        self.bank.lock().trigger(&self.tx, pad);
    }

    /// The whole PLAYER strip: music and the soundboard, each with its own fader and meter, then
    /// the same effects, send-to rows and level block as an input strip.
    pub fn show(&mut self, ui: &mut Ui, settings: &mut StripSettings, meters: &Meters, fine_tune_open: &mut bool, frame: ColumnFrame) {
        let ColumnFrame { geo, fader_height, any_solo } = frame;
        let silenced = settings.mute || (any_solo && !settings.solo);
        widgets::section(ui, "Music");
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(geo.left);
                self.music_controls(ui, geo);
            });
            if widgets::fader(ui, &mut self.music_db, widgets::FADER_MIN_HEIGHT) {
                let _ = self.tx.send(PlayerCommand::MusicGain(db_to_gain(self.music_db)));
            }
            widgets::meter(ui, self.status.music_peak.load(), widgets::FADER_MIN_HEIGHT, widgets::METER_WIDTH);
        });
        widgets::section(ui, "Soundboard");
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(geo.left);
                self.pad_grid(ui, geo);
            });
            if widgets::fader(ui, &mut self.pads_db, widgets::FADER_MIN_HEIGHT) {
                let _ = self.tx.send(PlayerCommand::PadsGain(db_to_gain(self.pads_db)));
            }
            widgets::meter(ui, self.status.pads_peak.load(), widgets::FADER_MIN_HEIGHT, widgets::METER_WIDTH);
        });
        widgets::section(ui, "Audio effects");
        fx_row(ui, settings, geo.fx_led);
        widgets::pan(ui, &mut settings.pan, geo.inner);
        widgets::section(ui, "Send to");
        routing_rows(ui, &mut settings.routing, PLAYER_STRIP, geo);
        widgets::section(ui, "Level");
        ui.horizontal_top(|ui| {
            ui.scope(|ui| {
                if silenced {
                    ui.set_opacity(0.45);
                }
                widgets::fader(ui, &mut settings.gain_db, fader_height);
                widgets::meter(ui, meters.strips[PLAYER_STRIP].load(), fader_height, widgets::METER_WIDTH);
            });
            widgets::level_column(ui, geo.side_button.x, |ui| state_column(ui, settings, geo, fine_tune_open));
        });
        if let Some(err) = &self.last_error {
            widgets::error_label(ui, err);
        }
        fine_tune_window(ui.ctx(), PLAYER_STRIP, settings, meters, fine_tune_open);
    }

    /// Load / play / stop / loop, the track name and clock, and the seek bar.
    fn music_controls(&mut self, ui: &mut Ui, geo: Geometry) {
        let playing = self.status.music_playing.load(Ordering::Relaxed);
        let loaded = self.music.is_some();
        ui.horizontal(|ui| {
            if widgets::button(ui, "LOAD…", widgets::TRANSPORT_BUTTON_SIZE, "MP3, WAV, FLAC, OGG (Vorbis) or M4A") {
                if let Some(path) = pick_audio_file() {
                    self.load_music(&path, true);
                }
            }
            ui.add_enabled_ui(loaded, |ui| {
                if widgets::button(ui, if playing { "PAUSE" } else { "PLAY" }, widgets::TRANSPORT_BUTTON_SIZE, "") {
                    let _ = self.tx.send(PlayerCommand::MusicPlayPause);
                }
                if widgets::button(ui, "STOP", widgets::TRANSPORT_BUTTON_SIZE, "") {
                    let _ = self.tx.send(PlayerCommand::MusicStop);
                }
            });
            if widgets::led(ui, &mut self.music_loop, "LOOP", COLOR_ACTIVE, widgets::TRANSPORT_BUTTON_SIZE, "Start again when the track ends.") {
                let _ = self.tx.send(PlayerCommand::MusicLoop(self.music_loop));
            }
        });
        let pos = self.status.music_position.load(Ordering::Relaxed) as f32;
        let len = self.status.music_length.load(Ordering::Relaxed).max(1) as f32;
        ui.horizontal(|ui| {
            match &self.music {
                Some((_, clip)) => widgets::value_label(ui, &widgets::shorten(&clip.name, 22)),
                None => widgets::hint(ui, "No track loaded"),
            }
            widgets::hint(ui, clock(pos, len));
        });
        let mut t = self.seek_target.unwrap_or(pos / len);
        ui.spacing_mut().slider_width = widgets::labelled_slider_width(geo.left);
        let slider = ui.add_enabled(loaded, egui::Slider::new(&mut t, 0.0..=1.0).show_value(false).text("seek")).on_hover_cursor(egui::CursorIcon::PointingHand);
        if slider.dragged() || slider.changed() {
            self.seek_target = Some(t);
        }
        if slider.drag_stopped() || (slider.changed() && !slider.dragged()) {
            if let Some(target) = self.seek_target.take() {
                let _ = self.tx.send(PlayerCommand::MusicSeek(target));
            }
        }
    }

    /// The three-by-three pad grid with "Stop all" beneath it (always drawn, greyed while
    /// nothing sounds, so the block never reflows).
    fn pad_grid(&mut self, ui: &mut Ui, geo: Geometry) {
        let mut to_assign: Option<usize> = None;
        let mut to_clear: Option<usize> = None;
        let mut to_play: Option<usize> = None;
        // One lock for the whole grid; the names are all it needs.
        let names: Vec<Option<String>> = self.bank.lock().pads.iter().map(|p| p.as_ref().map(|p| widgets::shorten(&p.clip.name, 8))).collect();
        egui::Grid::new("pads").spacing([PAD_SPACING, PAD_SPACING]).show(ui, |ui| {
            for (i, tip) in PAD_TIPS.iter().enumerate() {
                let filled = names[i].is_some();
                let name = names[i].clone().unwrap_or_else(|| "add".to_string());
                let fill = if filled { COLOR_ASSIGNED } else { widgets::COLOR_INSET };
                let text_color = if filled { widgets::on_color(fill) } else { widgets::COLOR_TEXT_MUTED };
                let text = egui::RichText::new(format!("{}  {name}", i + 1)).size(10.0).strong().color(text_color);
                let response = widgets::tile_button(ui, text, fill, geo.pad).on_hover_text(if filled { *tip } else { EMPTY_PAD_TIP });
                if response.clicked() {
                    if filled {
                        to_play = Some(i);
                    } else {
                        to_assign = Some(i);
                    }
                }
                response.context_menu(|ui| {
                    if ui.button("Choose file…").clicked() {
                        to_assign = Some(i);
                        ui.close_menu();
                    }
                    if ui.button("Clear pad").clicked() {
                        to_clear = Some(i);
                        ui.close_menu();
                    }
                });
                if (i + 1) % PAD_COLUMNS == 0 {
                    ui.end_row();
                }
            }
        });
        let sounding = self.status.pads_sounding.load(Ordering::Relaxed) > 0;
        ui.add_enabled_ui(sounding, |ui| {
            if widgets::button(ui, "STOP ALL", widgets::STOP_ALL_SIZE, "Silence every pad that is playing.") {
                let _ = self.tx.send(PlayerCommand::StopPads);
            }
        });
        if let Some(i) = to_play {
            self.trigger_pad(i);
        }
        if let Some(i) = to_clear {
            self.clear_pad(i);
        }
        if let Some(i) = to_assign {
            if let Some(path) = pick_audio_file() {
                self.assign_pad(i, &path);
            }
        }
    }
}

fn pick_audio_file() -> Option<PathBuf> {
    rfd::FileDialog::new().add_filter("Audio", &AUDIO_EXTENSIONS).pick_file()
}

fn clock(pos_frames: f32, len_frames: f32) -> String {
    let fmt = |frames: f32| {
        let secs = frames / SAMPLE_RATE as f32;
        format!("{}:{:02}", (secs / 60.0) as u32, (secs % 60.0) as u32)
    };
    format!("{} / {}", fmt(pos_frames), fmt(len_frames))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::player::Player;

    /// The pads row (the pads-volume slider, its trailing label, and the always-present "Stop
    /// all" button) must fit `geo.left`, the width `PlayerPanel::show` actually gives it, or
    /// "Stop all" is clipped by the floating Player window's edge.
    #[test]
    fn pads_row_fits_the_player_windows_left_column() {
        let geo = Geometry::for_column(widgets::PLAYER_WINDOW_WIDTH + 2.0 * widgets::PANEL_PADDING);
        let (_player, tx, status) = Player::new();
        let panel = std::cell::RefCell::new(PlayerPanel::new(tx, status));
        let width = std::cell::Cell::new(0.0);
        widgets::run_themed_test_ui(|ui| {
            ui.vertical(|ui| {
                ui.set_width(geo.left);
                panel.borrow_mut().pad_grid(ui, geo);
                width.set(ui.min_rect().width());
            });
        });
        assert!(width.get() <= geo.left + 1e-3, "pads row is {} px wide in the Player window's {} px left column", width.get(), geo.left);
    }

    /// `trigger` sends `PlayPad` for a loaded pad, and nothing at all for an empty one, so firing
    /// an unassigned hotkey pad is silently a no-op rather than playing stale or empty audio.
    #[test]
    fn trigger_sends_play_pad_for_a_loaded_pad_and_nothing_for_an_empty_one() {
        use crate::engine::player::{Clip, PlayerCommand};
        let (tx, rx) = crossbeam_channel::unbounded();
        let samples = std::sync::Arc::new(vec![0.25f32, -0.25]);
        let bank = PadBank {
            pads: vec![Some(Pad { path: PathBuf::from("clip.wav"), clip: Clip { name: "clip".into(), samples: samples.clone() } }), None],
        };

        bank.trigger(&tx, 0);
        match rx.try_recv().expect("the loaded pad must send a command") {
            PlayerCommand::PlayPad { pad, samples: sent } => {
                assert_eq!(pad, 0);
                assert!(std::sync::Arc::ptr_eq(&sent, &samples));
            }
            _ => panic!("expected PlayPad for the loaded pad"),
        }

        bank.trigger(&tx, 1);
        assert!(rx.try_recv().is_err(), "an empty pad must send nothing");

        bank.trigger(&tx, 99);
        assert!(rx.try_recv().is_err(), "an out-of-range pad must send nothing, not panic");
    }
}
