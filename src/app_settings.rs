//! Preferences about the app itself rather than the mix: tray behaviour, startup and hotkeys.
//! Saved as JSON beside eframe's own storage, so `main` can read them before a window exists.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The name eframe files the app's storage under; also the tray tooltip and login-item name.
pub const APP_ID: &str = "DHMIX";
const FILE_NAME: &str = "settings.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Closing the window hides it in the tray instead of quitting.
    pub close_to_tray: bool,
    /// The window starts hidden in the tray.
    pub start_in_tray: bool,
    /// Registered with the OS to launch when the user signs in.
    pub run_on_startup: bool,
    /// Ctrl+Alt+1..9 fire the soundboard pads.
    pub pad_hotkeys: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self { close_to_tray: false, start_in_tray: false, run_on_startup: false, pad_hotkeys: true }
    }
}

impl AppSettings {
    pub fn path() -> Option<PathBuf> {
        eframe::storage_dir(APP_ID).map(|dir| dir.join(FILE_NAME))
    }

    /// The saved settings, or the defaults when nothing was saved yet or the file is unreadable.
    pub fn load() -> Self {
        Self::path().map(|path| Self::load_from(&path)).unwrap_or_default()
    }

    pub fn load_from(path: &Path) -> Self {
        std::fs::read_to_string(path).ok().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path().ok_or_else(|| anyhow!("no settings directory on this system"))?;
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?).with_context(|| format!("write {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("dhmix-settings-{tag}-{}", std::process::id())).join(FILE_NAME)
    }

    #[test]
    fn defaults_keep_the_window_visible_and_the_pad_hotkeys_on() {
        let s = AppSettings::default();
        assert!(!s.close_to_tray && !s.start_in_tray && !s.run_on_startup);
        assert!(s.pad_hotkeys);
    }

    #[test]
    fn settings_round_trip_through_their_file() {
        let path = temp_path("round-trip");
        let saved = AppSettings { close_to_tray: true, start_in_tray: true, run_on_startup: false, pad_hotkeys: false };
        saved.save_to(&path).unwrap();
        assert_eq!(AppSettings::load_from(&path), saved);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn missing_or_partial_files_fall_back_to_defaults_per_field() {
        assert_eq!(AppSettings::load_from(Path::new("/definitely/not/here.json")), AppSettings::default());
        let path = temp_path("partial");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"close_to_tray": true}"#).unwrap();
        let loaded = AppSettings::load_from(&path);
        assert!(loaded.close_to_tray, "the field that was saved is read");
        assert!(loaded.pad_hotkeys, "a field missing from an older file keeps its default");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
