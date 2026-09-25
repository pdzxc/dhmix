//! Floating window listing running applications with audio sessions, which strip or bus each one
//! lands on, and a picker to move it to another device.

use super::widgets::{self, COLOR_ACTIVE, COLOR_VIRTUAL};
use crate::apps::{self, AppSession, Endpoint, Flow};
use crate::audio::devices::cable_partner;
use crate::audio::IoSettings;
use crate::{bus_name, strip_name, PLAYER_STRIP};
use std::borrow::Cow;

#[derive(Default)]
pub struct AppsPanel {
    pub open: bool,
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
    /// Draws the window. Returns true when the user moved an app, so the caller refreshes its
    /// snapshot right away instead of waiting for the next timer tick.
    pub fn show(&mut self, ctx: &egui::Context, io: &IoSettings, sessions: &[AppSession], endpoints: &[Endpoint]) -> bool {
        if !self.open {
            return false;
        }
        let mut moved = false;
        let mut open = self.open;
        egui::Window::new("Applications").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            if !apps::supported() {
                widgets::hint(ui, "Per-app routing works on Windows only. On this system the list is empty.");
            } else if sessions.is_empty() {
                widgets::hint(ui, "No application is playing or recording audio right now.");
            }
            let mut change: Option<(u32, Flow, Option<String>)> = None;
            egui::Grid::new("apps-grid").num_columns(4).spacing(widgets::TABLE_SPACING).striped(true).show(ui, |ui| {
                // Inline captions: a grid row may not add vertical space (egui panics if it does).
                widgets::caption(ui, "App");
                widgets::caption(ui, "Does");
                widgets::caption(ui, "Device now");
                widgets::caption(ui, "Send to");
                ui.end_row();
                for session in sessions {
                    widgets::value_label(ui, &session.name);
                    let (verb, color) = match session.flow {
                        Flow::Playback => ("plays", COLOR_ACTIVE),
                        Flow::Capture => ("records", COLOR_VIRTUAL),
                    };
                    ui.label(egui::RichText::new(verb).small().color(color));
                    let position = mixer_position(&session.device_name, session.flow, io).unwrap_or_else(|| "not in the mixer".into());
                    ui.label(format!("{} · {}", widgets::shorten(&session.device_name, 22), position));
                    let default = std::iter::once((Cow::Borrowed("Windows default"), None, false));
                    let endpoints = endpoints
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
                        moved = true;
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
        moved
    }
}

/// Names of the apps playing into the cable whose capture side is `strip_device`.
pub fn apps_feeding(sessions: &[AppSession], strip_device: Option<&str>) -> Vec<String> {
    let Some(device) = strip_device else { return Vec::new() };
    sessions
        .iter()
        .filter(|s| s.flow == Flow::Playback && cable_partner(&s.device_name).as_deref() == Some(device))
        .map(|s| s.name.clone())
        .collect()
}

/// Names of the apps recording from the cable whose playback side is `bus_device`.
pub fn apps_hearing(sessions: &[AppSession], bus_device: Option<&str>) -> Vec<String> {
    let Some(device) = bus_device else { return Vec::new() };
    sessions
        .iter()
        .filter(|s| s.flow == Flow::Capture && cable_partner(&s.device_name).as_deref() == Some(device))
        .map(|s| s.name.clone())
        .collect()
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
    fn apps_feeding_and_hearing_match_through_the_cable() {
        let sessions = vec![
            AppSession { pid: 1, name: "Game".into(), flow: Flow::Playback, device_name: "CABLE Input (VB-Audio Virtual Cable)".into() },
            AppSession { pid: 2, name: "Spotify".into(), flow: Flow::Playback, device_name: "Speakers (Realtek)".into() },
            AppSession { pid: 3, name: "OBS".into(), flow: Flow::Capture, device_name: "CABLE-A Output (VB-Audio Cable A)".into() },
        ];
        assert_eq!(apps_feeding(&sessions, Some("CABLE Output (VB-Audio Virtual Cable)")), vec!["Game".to_string()]);
        assert!(apps_feeding(&sessions, Some("Speakers (Realtek)")).is_empty());
        assert!(apps_feeding(&sessions, None).is_empty());
        assert_eq!(apps_hearing(&sessions, Some("CABLE-A Input (VB-Audio Cable A)")), vec!["OBS".to_string()]);
        assert!(apps_hearing(&sessions, Some("CABLE Input (VB-Audio Virtual Cable)")).is_empty());
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

#[cfg(test)]
mod render_tests {
    use super::*;

    /// The window must render on every platform, including where no sessions exist.
    #[test]
    fn applications_window_renders_without_panicking() {
        let ctx = egui::Context::default();
        let mut panel = AppsPanel { open: true, ..Default::default() };
        let io = IoSettings::empty();
        for _ in 0..3 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                panel.show(ctx, &io, &[], &[]);
            });
        }
    }

    /// Same, but with sessions and endpoints populated so the grid actually draws rows and the
    /// device picker has choices; this is the path with the most content to lay out.
    #[test]
    fn applications_window_with_sessions_and_endpoints_renders_without_panicking() {
        let ctx = egui::Context::default();
        let mut panel = AppsPanel { open: true, ..Default::default() };
        let io = IoSettings::empty();
        let sessions = vec![
            AppSession { pid: 1, name: "Game".into(), flow: Flow::Playback, device_name: "Speakers (Realtek)".into() },
            AppSession { pid: 2, name: "Discord".into(), flow: Flow::Capture, device_name: "Microphone (Realtek)".into() },
        ];
        let endpoints = vec![
            Endpoint { id: "spk".into(), name: "Speakers (Realtek)".into(), flow: Flow::Playback },
            Endpoint { id: "mic".into(), name: "Microphone (Realtek)".into(), flow: Flow::Capture },
        ];
        for _ in 0..3 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                panel.show(ctx, &io, &sessions, &endpoints);
            });
        }
    }
}
