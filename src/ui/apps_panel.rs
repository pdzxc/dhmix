//! Floating window listing running applications with audio sessions, which strip or bus each one
//! lands on, and a picker to move it to another device.

use super::widgets::{self, COLOR_ACTIVE, COLOR_VIRTUAL};
use crate::apps::{self, AppSession, Endpoint, Flow};
use crate::audio::devices::cable_partner;
use crate::audio::IoSettings;
use crate::{bus_name, strip_name, PLAYER_STRIP};
use std::borrow::Cow;
use std::time::{Duration, Instant};

const REFRESH_EVERY: Duration = Duration::from_secs(3);

#[derive(Default)]
pub struct AppsPanel {
    pub open: bool,
    sessions: Vec<AppSession>,
    endpoints: Vec<Endpoint>,
    last_refresh: Option<Instant>,
    error: Option<String>,
}

/// Where an app on `device` shows up inside the mixer, if anywhere.
pub fn mixer_position(device: &str, flow: Flow, io: &IoSettings) -> Option<String> {
    match flow {
        Flow::Playback => {
            let capture_side = cable_partner(device);
            let strip = io.strip_inputs.iter().position(|d| d.is_some() && d.as_deref() == capture_side.as_deref());
            if let Some(i) = strip.filter(|i| *i < PLAYER_STRIP) {
                return Some(format!("{} in the mixer", strip_name(i)));
            }
            let bus = io.bus_outputs.iter().position(|d| d.as_deref() == Some(device));
            bus.map(|b| format!("{} directly, bypassing the mixer", bus_name(b)))
        }
        Flow::Capture => {
            let render_side = cable_partner(device);
            let bus = io.bus_outputs.iter().position(|d| d.is_some() && d.as_deref() == render_side.as_deref());
            bus.map(|b| format!("hears {}", bus_name(b)))
        }
    }
}

/// Label for an endpoint in the picker: the plain device plus what it means in the mixer.
fn endpoint_label(endpoint: &Endpoint, io: &IoSettings) -> String {
    match mixer_position(&endpoint.name, endpoint.flow, io) {
        Some(pos) => format!("{} → {}", widgets::shorten(&endpoint.name, 26), pos),
        None => widgets::shorten(&endpoint.name, 40),
    }
}

impl AppsPanel {
    pub fn refresh(&mut self) {
        (self.sessions, self.endpoints) = apps::snapshot();
        self.last_refresh = Some(Instant::now());
    }

    fn refresh_if_stale(&mut self) {
        if self.last_refresh.is_none_or(|t| t.elapsed() > REFRESH_EVERY) {
            self.refresh();
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, io: &IoSettings) {
        if !self.open {
            return;
        }
        self.refresh_if_stale();
        let mut open = self.open;
        egui::Window::new("Applications").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            if !apps::supported() {
                widgets::hint(ui, "Per-app routing works on Windows only. On this system the list is empty.");
            } else if self.sessions.is_empty() {
                widgets::hint(ui, "No application is playing or recording audio right now.");
            }
            let mut change: Option<(u32, Flow, Option<String>)> = None;
            egui::Grid::new("apps-grid").num_columns(4).spacing(widgets::TABLE_SPACING).striped(true).show(ui, |ui| {
                widgets::section(ui, "App");
                widgets::section(ui, "Does");
                widgets::section(ui, "Device now");
                widgets::section(ui, "Send to");
                ui.end_row();
                for session in &self.sessions {
                    widgets::value_label(ui, &session.name);
                    let (verb, color) = match session.flow {
                        Flow::Playback => ("plays", COLOR_ACTIVE),
                        Flow::Capture => ("records", COLOR_VIRTUAL),
                    };
                    ui.label(egui::RichText::new(verb).small().color(color));
                    let position = mixer_position(&session.device_name, session.flow, io).unwrap_or_else(|| "not in the mixer".into());
                    ui.label(format!("{} · {}", widgets::shorten(&session.device_name, 22), position));
                    let default = std::iter::once((Cow::Borrowed("Windows default"), None, false));
                    let endpoints = self
                        .endpoints
                        .iter()
                        .filter(|e| e.flow == session.flow)
                        .map(|e| (Cow::Owned(endpoint_label(e, io)), Some(e.id.as_str()), e.name == session.device_name));
                    let id = ("route", session.pid, session.flow == Flow::Capture);
                    if let Some(endpoint) = widgets::choice_combo(ui, id, widgets::ROUTE_COMBO_WIDTH, "Choose…", default.chain(endpoints)) {
                        change = Some((session.pid, session.flow, endpoint.map(str::to_string)));
                    }
                    ui.end_row();
                }
            });
            if let Some((pid, flow, endpoint)) = change {
                match apps::set_app_device(pid, flow, endpoint.as_deref()) {
                    Ok(()) => {
                        self.error = None;
                        self.refresh();
                    }
                    Err(e) => self.error = Some(format!("{e:#}")),
                }
            }
            if let Some(err) = &self.error {
                widgets::error_label(ui, err);
            }
            widgets::hint(ui, "Moving an app takes effect the next time it starts a sound. Windows remembers it per app.");
        });
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NUM_HW_BUSES, NUM_HW_STRIPS};

    fn io() -> IoSettings {
        let mut io = IoSettings::empty();
        io.strip_inputs[NUM_HW_STRIPS] = Some("CABLE Output (VB-Audio Virtual Cable)".into());
        io.bus_outputs[0] = Some("Speakers (Realtek)".into());
        io.bus_outputs[NUM_HW_BUSES] = Some("CABLE-A Input (VB-Audio Cable A)".into());
        io
    }

    #[test]
    fn app_playing_into_a_cable_shows_the_virtual_strip_it_feeds() {
        let pos = mixer_position("CABLE Input (VB-Audio Virtual Cable)", Flow::Playback, &io());
        assert_eq!(pos.as_deref(), Some("VIRT 1 in the mixer"));
    }

    #[test]
    fn app_playing_straight_to_speakers_bypasses_the_mixer() {
        let pos = mixer_position("Speakers (Realtek)", Flow::Playback, &io());
        assert_eq!(pos.as_deref(), Some("A1 directly, bypassing the mixer"));
    }

    #[test]
    fn app_recording_from_a_cable_hears_the_bus_feeding_it() {
        let pos = mixer_position("CABLE-A Output (VB-Audio Cable A)", Flow::Capture, &io());
        assert_eq!(pos.as_deref(), Some("hears B1"));
        assert_eq!(mixer_position("Webcam Mic", Flow::Capture, &io()), None);
    }

    #[test]
    fn cable_capture_side_on_the_player_strip_is_not_reported() {
        let mut io = io();
        // PLAYER has no device of its own; a cable's capture side landing there must be ignored.
        io.strip_inputs[PLAYER_STRIP] = Some("CABLE-B Output (VB-Audio Cable B)".into());
        let pos = mixer_position("CABLE-B Input (VB-Audio Cable B)", Flow::Playback, &io);
        assert_eq!(pos, None);
    }

    #[test]
    fn app_playing_to_a_device_that_is_both_a_bus_output_and_a_cable_falls_back_to_the_bus() {
        // The device itself is a virtual cable, but no strip captures its (unmapped) partner side,
        // so the bus-output match must still be reported.
        let pos = mixer_position("CABLE-A Input (VB-Audio Cable A)", Flow::Playback, &io());
        assert_eq!(pos.as_deref(), Some("B1 directly, bypassing the mixer"));
    }
}
