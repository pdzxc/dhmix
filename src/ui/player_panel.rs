//! Soundboard pads and the music player, shown at the top of the PLAYER strip.

use super::widgets::{self, COLOR_ACTIVE, COLOR_ASSIGNED, LED_PAIR, PAD_SIZE, PAD_SPACING};
use crate::engine::player::{load_clip, Clip, PlayerCommand, PlayerStatus};
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

    pub fn show(&mut self, ui: &mut Ui) {
        self.music_controls(ui);
        self.pad_grid(ui);
        if let Some(err) = &self.last_error {
            widgets::error_label(ui, err);
        }
    }

    fn music_controls(&mut self, ui: &mut Ui) {
        widgets::section(ui, "Music");
        let playing = self.status.music_playing.load(Ordering::Relaxed);
        let loaded = self.music.is_some();
        ui.horizontal(|ui| {
            if ui.button("Load track…").on_hover_text("MP3, WAV, FLAC, OGG or M4A").clicked() {
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
            if widgets::led(ui, &mut self.music_loop, "LOOP", COLOR_ACTIVE, LED_PAIR, "Start again when the track ends.") {
                let _ = self.tx.send(PlayerCommand::MusicLoop(self.music_loop));
            }
        });
        match &self.music {
            Some((_, clip)) => widgets::value_label(ui, &widgets::shorten(&clip.name, 36)),
            None => widgets::hint(ui, "No track loaded. Load one, then send PLAYER to B1 so viewers hear it."),
        }

        let pos = self.status.music_position.load(Ordering::Relaxed) as f32;
        let len = self.status.music_length.load(Ordering::Relaxed).max(1) as f32;
        let mut t = self.seek_target.unwrap_or(pos / len);
        ui.spacing_mut().slider_width = widgets::PLAYER_SLIDER_WIDTH;
        let slider = ui.add_enabled(loaded, egui::Slider::new(&mut t, 0.0..=1.0).show_value(false).text(clock(pos, len)));
        if slider.dragged() || slider.changed() {
            self.seek_target = Some(t);
        }
        if slider.drag_stopped() || (slider.changed() && !slider.dragged()) {
            if let Some(target) = self.seek_target.take() {
                let _ = self.tx.send(PlayerCommand::MusicSeek(target));
            }
        }
        if ui.add(egui::Slider::new(&mut self.music_gain, 0.0..=1.5).text("music volume")).changed() {
            let _ = self.tx.send(PlayerCommand::MusicGain(self.music_gain));
        }
    }

    fn pad_grid(&mut self, ui: &mut Ui) {
        widgets::section(ui, "Soundboard");
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = widgets::PAD_SLIDER_WIDTH;
            ui.add(egui::Slider::new(&mut self.pad_gain, 0.0..=1.5).text("pad volume"));
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
                    Some(p) => widgets::shorten(&p.clip.name, 11),
                    None => "Add sound".to_string(),
                };
                let fill = if filled { COLOR_ASSIGNED } else { widgets::COLOR_INSET };
                let text_color = if filled { widgets::on_color(fill) } else { widgets::COLOR_TEXT_MUTED };
                let text = egui::RichText::new(format!("{}  {name}", i + 1)).size(10.0).strong().color(text_color);
                let response = widgets::tile_button(ui, text, fill, PAD_SIZE).on_hover_text(if filled { *tip } else { EMPTY_PAD_TIP });
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
