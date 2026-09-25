//! Soundboard pads and the music player, shown inside the PLAYER strip.

use crate::engine::player::{load_clip, Clip, PlayerCommand, PlayerStatus};
use crate::preset::NUM_PADS;
use crate::SAMPLE_RATE;
use crossbeam_channel::Sender;
use super::widgets::{self, COLOR_ACTIVE, COLOR_ASSIGNED};
use egui::{Ui, Vec2};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

const PAD_COLUMNS: usize = 3;
const PAD_SIZE: Vec2 = Vec2::new(96.0, 40.0);
const AUDIO_EXTENSIONS: [&str; 7] = ["mp3", "wav", "flac", "ogg", "m4a", "aac", "mp4"];

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

    pub fn show(&mut self, ui: &mut Ui) {
        self.music_controls(ui);
        ui.separator();
        self.pad_grid(ui);
        if let Some(err) = &self.last_error {
            widgets::error_label(ui, err);
        }
    }

    fn music_controls(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new("Music").strong());
        ui.horizontal(|ui| {
            if ui.button("Load…").clicked() {
                if let Some(path) = pick_audio_file() {
                    self.load_music(&path, true);
                }
            }
            let playing = self.status.music_playing.load(Ordering::Relaxed);
            if ui.add_enabled(self.music.is_some(), egui::Button::new(if playing { "⏸" } else { "▶" })).clicked() {
                let _ = self.tx.send(PlayerCommand::MusicPlayPause);
            }
            if ui.add_enabled(self.music.is_some(), egui::Button::new("⏹")).clicked() {
                let _ = self.tx.send(PlayerCommand::MusicStop);
            }
            if widgets::toggle(ui, &mut self.music_loop, "LOOP", COLOR_ACTIVE) {
                let _ = self.tx.send(PlayerCommand::MusicLoop(self.music_loop));
            }
        });
        let name = self.music.as_ref().map(|(_, c)| c.name.as_str()).unwrap_or("no track loaded");
        ui.label(widgets::shorten(name, 40));

        let pos = self.status.music_position.load(Ordering::Relaxed) as f32;
        let len = self.status.music_length.load(Ordering::Relaxed).max(1) as f32;
        let mut t = self.seek_target.unwrap_or(pos / len);
        ui.spacing_mut().slider_width = widgets::PLAYER_SLIDER_WIDTH;
        let slider = ui.add(egui::Slider::new(&mut t, 0.0..=1.0).show_value(false).text(clock(pos, len)));
        if slider.dragged() || slider.changed() {
            self.seek_target = Some(t);
        }
        if slider.drag_stopped() || (slider.changed() && !slider.dragged()) {
            if let Some(target) = self.seek_target.take() {
                let _ = self.tx.send(PlayerCommand::MusicSeek(target));
            }
        }
        if ui.add(egui::Slider::new(&mut self.music_gain, 0.0..=1.5).text("music vol")).changed() {
            let _ = self.tx.send(PlayerCommand::MusicGain(self.music_gain));
        }
    }

    fn pad_grid(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Soundboard").strong());
            if ui.small_button("stop all").clicked() {
                let _ = self.tx.send(PlayerCommand::StopPads);
            }
        });
        ui.add(egui::Slider::new(&mut self.pad_gain, 0.0..=1.5).text("pad vol"));
        let mut to_assign: Option<usize> = None;
        let mut to_clear: Option<usize> = None;
        let mut to_play: Option<usize> = None;
        egui::Grid::new("pads").spacing([4.0, 4.0]).show(ui, |ui| {
            for i in 0..NUM_PADS {
                ui.vertical(|ui| {
                    let label = match &self.pads[i] {
                        Some(p) => format!("{}\n{}", i + 1, widgets::shorten(&p.clip.name, 12)),
                        None => format!("{}\n+", i + 1),
                    };
                    let filled = self.pads[i].is_some();
                    let fill = if filled { COLOR_ASSIGNED } else { ui.visuals().widgets.inactive.bg_fill };
                    let button = egui::Button::new(label).min_size(PAD_SIZE).fill(fill);
                    let response = ui.add(button);
                    if response.clicked() {
                        if filled {
                            to_play = Some(i);
                        } else {
                            to_assign = Some(i);
                        }
                    }
                    response.context_menu(|ui| {
                        if ui.button("Assign file…").clicked() {
                            to_assign = Some(i);
                            ui.close_menu();
                        }
                        if ui.button("Clear").clicked() {
                            to_clear = Some(i);
                            ui.close_menu();
                        }
                    });
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
        widgets::hint(ui, "Click a pad to play, right-click to change. Ctrl+Alt+1-9 triggers pads globally.");
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
