//! Soundboard pads and the music player, shown at the top of the PLAYER strip.

use super::strip_panel::{fine_tune_window, fx_row, pan_slider_width, routing_rows, state_row};
use super::widgets::{self, ColumnFrame, Geometry, COLOR_ACTIVE, COLOR_ASSIGNED, PAD_SPACING};
use crate::engine::player::{load_clip, Clip, PlayerCommand, PlayerStatus};
use crate::engine::{Meters, StripSettings};
use crate::PLAYER_STRIP;
use crate::preset::NUM_PADS;
use crate::SAMPLE_RATE;
use crossbeam_channel::Sender;
use egui::Ui;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

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

pub struct PlayerPanel {
    pub pads: Vec<Option<Pad>>,
    pub music: Option<(PathBuf, Clip)>,
    pub music_loop: bool,
    pub music_gain: f32,
    pub pad_gain: f32,
    pub last_error: Option<String>,
    tx: Sender<PlayerCommand>,
    status: std::sync::Arc<PlayerStatus>,
    seek_target: Option<f32>,
}

impl PlayerPanel {
    pub fn new(tx: Sender<PlayerCommand>, status: std::sync::Arc<PlayerStatus>) -> Self {
        Self {
            pads: (0..NUM_PADS).map(|_| None).collect(),
            music: None,
            music_loop: false,
            music_gain: 1.0,
            pad_gain: 1.0,
            last_error: None,
            tx,
            status,
            seek_target: None,
        }
    }

    pub fn pad_paths(&self) -> Vec<Option<PathBuf>> {
        self.pads.iter().map(|p| p.as_ref().map(|p| p.path.clone())).collect()
    }

    pub fn music_path(&self) -> Option<PathBuf> {
        self.music.as_ref().map(|(p, _)| p.clone())
    }

    pub fn assign_pad(&mut self, pad: usize, path: &Path) {
        match load_clip(path) {
            Ok(clip) => self.pads[pad] = Some(Pad { path: path.to_path_buf(), clip }),
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
        if let Some(Some(p)) = self.pads.get(pad) {
            let _ = self.tx.send(PlayerCommand::PlayPad { pad, samples: p.clip.samples.clone(), gain: self.pad_gain });
        }
    }

    /// The whole PLAYER strip: music on top, pads beside the fader, then sends and state.
    pub fn show(&mut self, ui: &mut Ui, settings: &mut StripSettings, meters: &Meters, fine_tune_open: &mut bool, frame: ColumnFrame) {
        let ColumnFrame { geo, fader_height, any_solo } = frame;
        self.music_controls(ui, geo);
        widgets::section(ui, "Soundboard");
        let silenced = settings.mute || (any_solo && !settings.solo);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(geo.left);
                self.pad_grid(ui, geo);
                ui.add_space(widgets::SECTION_GAP);
                widgets::section(ui, "Send to");
                routing_rows(ui, &mut settings.routing, PLAYER_STRIP, geo);
                ui.add_space(widgets::SECTION_GAP);
                state_row(ui, settings, geo);
                fx_row(ui, settings, geo);
            });
            ui.scope(|ui| {
                if silenced {
                    ui.set_opacity(0.45);
                }
                widgets::fader(ui, &mut settings.gain_db, fader_height);
                widgets::meter(ui, meters.strips[PLAYER_STRIP].load(), fader_height);
            });
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = pan_slider_width(geo.inner);
            ui.add(egui::Slider::new(&mut settings.pan, -1.0..=1.0).show_value(false).text("pan"))
                .on_hover_text("Left / right balance. Double-click to centre.");
            if ui.small_button("Fine-tune…").on_hover_text("All effect parameters in a separate window").clicked() {
                *fine_tune_open = !*fine_tune_open;
            }
        });
        if let Some(err) = &self.last_error {
            widgets::error_label(ui, err);
        }
        fine_tune_window(ui.ctx(), PLAYER_STRIP, settings, meters, fine_tune_open);
    }

    fn music_controls(&mut self, ui: &mut Ui, geo: Geometry) {
        widgets::section(ui, "Music");
        let playing = self.status.music_playing.load(Ordering::Relaxed);
        let loaded = self.music.is_some();
        ui.horizontal(|ui| {
            if ui.button("Load…").on_hover_text("MP3, WAV, FLAC, OGG or M4A").clicked() {
                if let Some(path) = pick_audio_file() {
                    self.load_music(&path, true);
                }
            }
            let play_label = if playing { "Pause" } else { "Play" };
            if ui.add_enabled(loaded, egui::Button::new(play_label)).clicked() {
                let _ = self.tx.send(PlayerCommand::MusicPlayPause);
            }
            if ui.add_enabled(loaded, egui::Button::new("Stop")).clicked() {
                let _ = self.tx.send(PlayerCommand::MusicStop);
            }
            if widgets::led(ui, &mut self.music_loop, "LOOP", COLOR_ACTIVE, geo.led_triple, "Start again when the track ends.") {
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
        ui.spacing_mut().slider_width = pan_slider_width(geo.inner);
        let slider = ui.add_enabled(loaded, egui::Slider::new(&mut t, 0.0..=1.0).show_value(false).text("seek"));
        if slider.dragged() || slider.changed() {
            self.seek_target = Some(t);
        }
        if slider.drag_stopped() || (slider.changed() && !slider.dragged()) {
            if let Some(target) = self.seek_target.take() {
                let _ = self.tx.send(PlayerCommand::MusicSeek(target));
            }
        }
        if ui.add(egui::Slider::new(&mut self.music_gain, 0.0..=1.5).show_value(false).text("music vol")).changed() {
            let _ = self.tx.send(PlayerCommand::MusicGain(self.music_gain));
        }
    }

    fn pad_grid(&mut self, ui: &mut Ui, geo: Geometry) {
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = pan_slider_width(geo.left);
            ui.add(egui::Slider::new(&mut self.pad_gain, 0.0..=1.5).show_value(false).text("pad vol"));
            if self.status.pads_sounding.load(Ordering::Relaxed) > 0 && ui.small_button("Stop all").clicked() {
                let _ = self.tx.send(PlayerCommand::StopPads);
            }
        });
        let mut to_assign: Option<usize> = None;
        let mut to_clear: Option<usize> = None;
        let mut to_play: Option<usize> = None;
        egui::Grid::new("pads").spacing([PAD_SPACING, PAD_SPACING]).show(ui, |ui| {
            for (i, tip) in PAD_TIPS.iter().enumerate() {
                let filled = self.pads[i].is_some();
                let name = match &self.pads[i] {
                    Some(p) => widgets::shorten(&p.clip.name, 8),
                    None => "add".to_string(),
                };
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
        if let Some(i) = to_play {
            self.trigger_pad(i);
        }
        if let Some(i) = to_clear {
            self.pads[i] = None;
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
