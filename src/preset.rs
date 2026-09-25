//! Everything worth saving between sessions: mixer settings, device choices, soundboard pads.

use crate::audio::IoSettings;
use crate::dsp::compressor::CompressorSettings;
use crate::dsp::echo::EchoSettings;
use crate::dsp::eq::EqSettings;
use crate::dsp::gate::GateSettings;
use crate::dsp::reverb::ReverbSettings;
use crate::engine::{BusSettings, MixSettings, StripSettings};
use crate::{NUM_BUSES, NUM_STRIPS, PLAYER_STRIP};
use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::path::{Path, PathBuf};

pub const NUM_PADS: usize = 9;

#[derive(Clone, Debug, PartialEq, Serialize)]
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
        serde_json::from_str(&json).context("preset is not valid JSON")
    }
}

const LEGACY_NUM_STRIPS: usize = 7;
const LEGACY_NUM_BUSES: usize = 7;
const LEGACY_PLAYER_STRIP: usize = 6;
const LEGACY_REMOVED_VIRTUAL_STRIP: usize = 5;
/// Old bus slots that survive: A1..A3 and B1..B2 (old A4 and A5 are dropped).
const LEGACY_KEPT_BUSES: [usize; 5] = [0, 1, 2, 5, 6];

fn select_indices<T>(values: Vec<T>, indices: &[usize]) -> Vec<T> {
    let mut slots: Vec<_> = values.into_iter().map(Some).collect();
    indices.iter().filter_map(|&index| slots.get_mut(index)?.take()).collect()
}

#[derive(Deserialize, Serialize)]
struct PresetWire {
    mix: MixSettingsWire,
    io: IoSettings,
    pads: Vec<Option<PathBuf>>,
    music: Option<PathBuf>,
    hotkey_strip: usize,
}

#[derive(Deserialize, Serialize)]
struct MixSettingsWire {
    strips: Vec<StripSettingsWire>,
    buses: Vec<BusSettings>,
}

#[derive(Deserialize, Serialize)]
struct StripSettingsWire {
    gain_db: f32,
    mute: bool,
    solo: bool,
    mono: bool,
    pan: f32,
    routing: Vec<bool>,
    denoise: bool,
    gate: GateSettings,
    eq: EqSettings,
    comp: CompressorSettings,
    echo: EchoSettings,
    reverb: ReverbSettings,
}

impl StripSettingsWire {
    fn into_current(self) -> StripSettings {
        let routing = if self.routing.len() == LEGACY_NUM_BUSES {
            select_indices(self.routing, &LEGACY_KEPT_BUSES)
        } else {
            self.routing
        };
        let mut current_routing = [false; NUM_BUSES];
        for (target, source) in current_routing.iter_mut().zip(routing) {
            *target = source;
        }
        StripSettings {
            gain_db: self.gain_db,
            mute: self.mute,
            solo: self.solo,
            mono: self.mono,
            pan: self.pan,
            routing: current_routing,
            denoise: self.denoise,
            gate: self.gate,
            eq: self.eq,
            comp: self.comp,
            echo: self.echo,
            reverb: self.reverb,
        }
    }
}

impl Preset {
    /// Converts the former 3-virtual-input / 5-hardware-output wire layout. B1/B2 must be
    /// selected from their old positions, not obtained by truncation, and PLAYER must move over
    /// the removed VIRT 3 slot. This runs for exported JSON and eframe's automatic RON storage.
    fn from_wire(mut wire: PresetWire) -> Self {
        let legacy_strips = wire.mix.strips.len() == LEGACY_NUM_STRIPS
            || wire.io.strip_inputs.len() == LEGACY_NUM_STRIPS;
        let strips = if wire.mix.strips.len() == LEGACY_NUM_STRIPS {
            select_indices(wire.mix.strips, &[0, 1, 2, 3, 4, LEGACY_PLAYER_STRIP])
        } else {
            wire.mix.strips
        };
        let buses = if wire.mix.buses.len() == LEGACY_NUM_BUSES {
            select_indices(wire.mix.buses, &LEGACY_KEPT_BUSES)
        } else {
            wire.mix.buses
        };

        let mut current_strips = [StripSettings::default(); NUM_STRIPS];
        for (target, source) in current_strips.iter_mut().zip(strips) {
            *target = source.into_current();
        }
        let mut current_buses = [BusSettings::default(); NUM_BUSES];
        for (target, source) in current_buses.iter_mut().zip(buses) {
            *target = source;
        }

        if wire.io.strip_inputs.len() == LEGACY_NUM_STRIPS {
            // PLAYER is internally generated and never owns an input device. Drop the removed
            // VIRT 3 endpoint and let resize add PLAYER's empty slot.
            wire.io.strip_inputs = select_indices(wire.io.strip_inputs, &[0, 1, 2, 3, 4]);
        }
        if wire.io.bus_outputs.len() == LEGACY_NUM_BUSES {
            wire.io.bus_outputs = select_indices(wire.io.bus_outputs, &LEGACY_KEPT_BUSES);
        }
        wire.io.strip_inputs.resize(NUM_STRIPS, None);
        wire.io.bus_outputs.resize(NUM_BUSES, None);
        wire.pads.resize(NUM_PADS, None);

        if legacy_strips {
            wire.hotkey_strip = match wire.hotkey_strip {
                LEGACY_PLAYER_STRIP => PLAYER_STRIP,
                LEGACY_REMOVED_VIRTUAL_STRIP => 0,
                index => index,
            };
        }
        if wire.hotkey_strip >= NUM_STRIPS {
            wire.hotkey_strip = 0;
        }

        Self {
            mix: MixSettings { strips: current_strips, buses: current_buses }.clamped(),
            io: wire.io,
            pads: wire.pads,
            music: wire.music,
            hotkey_strip: wire.hotkey_strip,
        }
    }
}

impl<'de> Deserialize<'de> for Preset {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self::from_wire(PresetWire::deserialize(deserializer)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemoryStorage(Option<String>);

    impl eframe::Storage for MemoryStorage {
        fn get_string(&self, _key: &str) -> Option<String> {
            self.0.clone()
        }

        fn set_string(&mut self, _key: &str, value: String) {
            self.0 = Some(value);
        }

        fn flush(&mut self) {}
    }

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

    #[test]
    fn load_migrates_legacy_virtual_inputs_and_outputs_by_name() {
        let mut legacy = serde_json::to_value(Preset::default()).unwrap();
        let strips = legacy["mix"]["strips"].as_array_mut().unwrap();
        let removed_virtual = serde_json::to_value(crate::engine::StripSettings::default()).unwrap();
        strips.insert(5, removed_virtual);
        for strip in strips.iter_mut() {
            let routing = strip["routing"].as_array_mut().unwrap();
            routing.insert(3, serde_json::json!(false));
            routing.insert(4, serde_json::json!(false));
        }
        strips[5]["gain_db"] = serde_json::json!(-5.0); // removed VIRT 3
        strips[6]["gain_db"] = serde_json::json!(-6.0); // PLAYER moves from 6 to 5
        strips[0]["routing"] = serde_json::json!([true, false, false, true, false, true, false]);

        let buses = legacy["mix"]["buses"].as_array_mut().unwrap();
        let removed_bus = serde_json::to_value(crate::engine::BusSettings::default()).unwrap();
        buses.insert(3, removed_bus.clone());
        buses.insert(4, removed_bus);
        for (index, bus) in buses.iter_mut().enumerate() {
            bus["gain_db"] = serde_json::json!(-(index as f32));
        }
        legacy["io"]["strip_inputs"] =
            serde_json::json!(["HW 1", "HW 2", "HW 3", "VIRT 1", "VIRT 2", "VIRT 3", "PLAYER"]);
        legacy["io"]["bus_outputs"] = serde_json::json!(["A1", "A2", "A3", "A4", "A5", "B1", "B2"]);
        legacy["hotkey_strip"] = serde_json::json!(6);

        let dir = std::env::temp_dir().join(format!("streammix-preset-legacy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("legacy.json");
        std::fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

        let loaded = Preset::load(&path).unwrap();
        assert_eq!(loaded.mix.strips.len(), 6);
        assert_eq!(
            loaded.mix.strips[5].gain_db, -6.0,
            "PLAYER is retained after VIRT 3 is removed"
        );
        assert_eq!(
            loaded.io.strip_inputs,
            [Some("HW 1"), Some("HW 2"), Some("HW 3"), Some("VIRT 1"), Some("VIRT 2"), None]
                .map(|name| name.map(str::to_owned))
        );
        assert_eq!(loaded.mix.buses.len(), 5);
        assert_eq!(loaded.mix.buses[3].gain_db, -5.0, "legacy B1 moves after A3");
        assert_eq!(loaded.mix.buses[4].gain_db, -6.0, "legacy B2 moves after B1");
        assert_eq!(
            loaded.io.bus_outputs,
            ["A1", "A2", "A3", "B1", "B2"].map(|s| Some(s.into()))
        );
        assert_eq!(loaded.mix.strips[0].routing, [true, false, false, true, false]);
        assert_eq!(loaded.hotkey_strip, 5, "hotkey follows PLAYER to its new strip index");

        let mut storage = MemoryStorage::default();
        let legacy_wire: PresetWire = serde_json::from_value(legacy).unwrap();
        eframe::set_value(&mut storage, "preset", &legacy_wire);
        let restored: Preset = eframe::get_value(&storage, "preset").unwrap();
        assert_eq!(restored, loaded, "automatic RON storage uses the same migration");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn legacy_routing_drops_removed_hardware_buses_and_moves_virtual_buses_to_their_new_slots() {
        // Old 7-bus layout: A1..A5 at 0..4, B1 at 5, B2 at 6. A4/A5 (indices 3, 4) are the
        // hardware outs that got removed; B1/B2 must land on the new indices 3 and 4.
        let legacy_routing = vec![false, false, false, true, true, true, true];
        let migrated = select_indices(legacy_routing, &LEGACY_KEPT_BUSES);
        assert_eq!(
            migrated,
            vec![false, false, false, true, true],
            "A4/A5 routing is dropped; B1 -> new index 3, B2 -> new index 4"
        );
    }

    #[test]
    fn select_indices_with_out_of_range_indices_does_not_panic() {
        // select_indices is only ever called with indices known to be in range for the legacy
        // wire layout, but it must not panic if that ever changes: an out-of-range index is
        // simply skipped rather than producing a value.
        let values = vec!["a", "b", "c"];
        let result = select_indices(values, &[0, 5, 2, 99]);
        assert_eq!(result, vec!["a", "c"], "out-of-range indices are dropped, not panicked on");
    }
}
