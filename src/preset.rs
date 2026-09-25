//! Everything worth saving between sessions: mixer settings, device choices, soundboard pads.

use crate::audio::IoSettings;
use crate::engine::MixSettings;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const NUM_PADS: usize = 9;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub mix: MixSettings,
    pub io: IoSettings,
    pub pads: Vec<Option<PathBuf>>,
    pub music: Option<PathBuf>,
    /// Strip that the global mute hotkey toggles.
    pub hotkey_strip: usize,
}

impl Default for Preset {
    fn default() -> Self {
        Self { mix: MixSettings::default(), io: IoSettings::empty(), pads: vec![None; NUM_PADS], music: None, hotkey_strip: 0 }
    }
}

impl Preset {
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json).with_context(|| format!("write {}", path.display()))
    }

    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let mut preset: Preset = serde_json::from_str(&json).context("preset is not valid JSON")?;
        preset.mix = preset.mix.clamped();
        preset.pads.resize(NUM_PADS, None);
        Ok(preset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_round_trips_through_json() {
        let mut p = Preset::default();
        p.mix.strips[2].gain_db = -4.5;
        p.mix.strips[2].eq.bass_db = 3.0;
        p.io.strip_inputs[0] = Some("Mic".into());
        p.pads[3] = Some(PathBuf::from("C:/sounds/airhorn.mp3"));
        let dir = std::env::temp_dir().join(format!("streammix-preset-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("p.json");
        p.save(&path).unwrap();
        assert_eq!(Preset::load(&path).unwrap(), p);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_pads_shorter_than_nine_are_padded_with_none() {
        let p = Preset {
            pads: vec![Some(PathBuf::from("a.mp3")), None, Some(PathBuf::from("c.mp3"))],
            ..Preset::default()
        };
        let dir = std::env::temp_dir().join(format!("streammix-preset-short-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("short.json");
        p.save(&path).unwrap();

        let loaded = Preset::load(&path).unwrap();
        assert_eq!(loaded.pads.len(), NUM_PADS, "short pads array is padded out to NUM_PADS");
        assert_eq!(loaded.pads[0], Some(PathBuf::from("a.mp3")));
        assert_eq!(loaded.pads[1], None);
        assert_eq!(loaded.pads[2], Some(PathBuf::from("c.mp3")));
        assert_eq!(loaded.pads[3], None, "newly added slots are None");
        let _ = std::fs::remove_dir_all(dir);
    }


    #[test]
    fn load_clamps_out_of_range_values_to_the_ui_ranges() {
        let mut p = Preset::default();
        p.mix.strips[0].gain_db = 400.0;
        p.mix.strips[0].pan = -7.0;
        p.mix.strips[0].comp.ratio = -3.0;
        p.mix.buses[1].gain_db = -999.0;
        let dir = std::env::temp_dir().join(format!("streammix-clamp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("p.json");
        p.save(&path).unwrap();
        let loaded = Preset::load(&path).unwrap();
        assert_eq!(loaded.mix.strips[0].gain_db, 12.0);
        assert_eq!(loaded.mix.strips[0].pan, -1.0);
        assert_eq!(loaded.mix.strips[0].comp.ratio, 1.0);
        assert_eq!(loaded.mix.buses[1].gain_db, -60.0);
        let _ = std::fs::remove_dir_all(dir);
    }
}
