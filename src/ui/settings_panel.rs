//! The Settings window: tray behaviour, run on startup and the pad hotkeys.

use super::widgets::{self, COLOR_ACTIVE};
use crate::app_settings::AppSettings;

pub struct SettingsPanel {
    pub open: bool,
    /// Why the last "run on startup" change was refused by the OS, if it was.
    pub startup_error: Option<String>,
}

impl SettingsPanel {
    /// Draws the window. Returns true when a setting changed, so the caller applies and saves it.
    pub fn show(&mut self, ctx: &egui::Context, settings: &mut AppSettings, tray_available: bool, mute_hotkey_strip: &str) -> bool {
        if !self.open {
            return false;
        }
        let mut changed = false;
        let mut open = self.open;
        egui::Window::new("Settings").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            ui.set_min_width(widgets::SETTINGS_WINDOW_WIDTH);
            widgets::section(ui, "Window");
            changed |= toggle(ui, &mut settings.close_to_tray, "CLOSE TO TRAY", "Closing the window keeps DHMIX mixing in the tray. Quit from the tray menu.");
            changed |= toggle(ui, &mut settings.start_in_tray, "START IN TRAY", "Start hidden; click the tray icon to open the window.");
            if !tray_available {
                widgets::hint(ui, "No system tray on this system, so the window always shows.");
            }
            widgets::section(ui, "Startup");
            changed |= toggle(ui, &mut settings.run_on_startup, "RUN ON STARTUP", "Launch DHMIX when you sign in.");
            if let Some(err) = &self.startup_error {
                widgets::error_label(ui, err);
            }
            widgets::section(ui, "Hotkeys");
            changed |= toggle(ui, &mut settings.pad_hotkeys, "PAD HOTKEYS", "Ctrl+Alt+1 to 9 fire the soundboard pads, even while a game has focus.");
            widgets::hint(ui, format!("Ctrl+Alt+M always mutes {mute_hotkey_strip}."));
        });
        self.open = open;
        changed
    }
}

/// One setting: an LED toggle with what it does beside it.
fn toggle(ui: &mut egui::Ui, on: &mut bool, label: &str, what: &str) -> bool {
    ui.horizontal(|ui| {
        let changed = widgets::led(ui, on, label, COLOR_ACTIVE, widgets::SETTINGS_TOGGLE_SIZE, "");
        widgets::row_hint(ui, what);
        changed
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_window_renders_without_panicking() {
        let ctx = egui::Context::default();
        let mut panel = SettingsPanel { open: true, startup_error: Some("refused".into()) };
        let mut settings = AppSettings::default();
        for tray in [true, false] {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                panel.show(ctx, &mut settings, tray, "HW 1");
            });
        }
        assert!(panel.open);
    }
}
