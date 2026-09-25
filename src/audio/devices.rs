//! Endpoint enumeration. On Windows this is WASAPI, so virtual cables (VB-CABLE "CABLE Input" /
//! "CABLE Output") show up here like any other device. A cable renamed in Windows Sound settings
//! to "DHMIX Input" / "DHMIX Output" is recognised too.

use cpal::traits::{DeviceTrait, HostTrait};

#[derive(Clone, Debug, Default)]
pub struct DeviceList {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

impl DeviceList {
    /// Names that look like a virtual cable's capture side, offered first for VIRT strips.
    pub fn virtual_inputs(&self) -> Vec<String> {
        self.inputs.iter().filter(|n| is_virtual_name(n)).cloned().collect()
    }
}

pub fn is_virtual_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["cable", "virtual", "vb-audio", "voicemeeter", "blackhole", "loopback", "dhmix"].iter().any(|k| n.contains(k))
}

/// The other end of a virtual cable: VB-CABLE names its playback side "... Input" and its
/// recording side "... Output". Returns `None` for devices that are not cables.
pub fn cable_partner(name: &str) -> Option<String> {
    if !is_virtual_name(name) {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    for (word, partner) in [("input", "Output"), ("output", "Input")] {
        if let Some(i) = lower.find(word) {
            return Some(format!("{}{partner}{}", &name[..i], &name[i + word.len()..]));
        }
    }
    None
}

pub fn list_devices() -> DeviceList {
    let host = cpal::default_host();
    fn names(devices: impl Iterator<Item = cpal::Device>) -> Vec<String> {
        devices.filter_map(|dev| dev.name().ok()).collect()
    }
    DeviceList {
        inputs: host.input_devices().map(names).unwrap_or_default(),
        outputs: host.output_devices().map(names).unwrap_or_default(),
    }
}

pub fn find_input(name: &str) -> Option<cpal::Device> {
    cpal::default_host().input_devices().ok()?.find(|d| d.name().map(|n| n == name).unwrap_or(false))
}

pub fn find_output(name: &str) -> Option<cpal::Device> {
    cpal::default_host().output_devices().ok()?.find(|d| d.name().map(|n| n == name).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_cable_names_are_recognised() {
        assert!(is_virtual_name("CABLE Output (VB-Audio Virtual Cable)"));
        assert!(is_virtual_name("DHMIX Input"), "a cable renamed after the app still counts");
        assert_eq!(cable_partner("DHMIX Input").as_deref(), Some("DHMIX Output"));
        assert!(is_virtual_name("Voicemeeter Out B1"));
        assert!(!is_virtual_name("Realtek High Definition Audio"));
    }

    #[test]
    fn cable_partner_swaps_input_and_output_sides() {
        assert_eq!(cable_partner("CABLE Input (VB-Audio Virtual Cable)").as_deref(), Some("CABLE Output (VB-Audio Virtual Cable)"));
        assert_eq!(cable_partner("CABLE-A Output (VB-Audio Cable A)").as_deref(), Some("CABLE-A Input (VB-Audio Cable A)"));
        assert_eq!(cable_partner("DHMIX Output").as_deref(), Some("DHMIX Input"));
        assert_eq!(cable_partner("Realtek Speakers"), None);
    }

    #[test]
    fn virtual_inputs_keeps_only_cable_like_names_in_original_order() {
        let devices = DeviceList {
            inputs: vec![
                "Realtek High Definition Audio".to_string(),
                "CABLE Output (VB-Audio Virtual Cable)".to_string(),
                "Built-in Microphone".to_string(),
                "Voicemeeter Out B1".to_string(),
            ],
            outputs: vec![],
        };
        assert_eq!(
            devices.virtual_inputs(),
            vec!["CABLE Output (VB-Audio Virtual Cable)".to_string(), "Voicemeeter Out B1".to_string()]
        );
    }

    #[test]
    fn virtual_inputs_is_empty_when_no_devices_match() {
        let devices = DeviceList { inputs: vec!["Realtek High Definition Audio".to_string()], outputs: vec![] };
        assert!(devices.virtual_inputs().is_empty());
    }

    #[test]
    fn cable_partner_handles_names_containing_both_words() {
        // "Input" is matched first, so the first occurrence wins even though "Output" also appears.
        assert_eq!(cable_partner("Input to Output Cable (VB-Audio Virtual Cable)").as_deref(), Some("Output to Output Cable (VB-Audio Virtual Cable)"));
    }

    #[test]
    fn cable_partner_is_none_for_virtual_names_containing_neither_word() {
        assert_eq!(cable_partner("Voicemeeter Out B1"), None);
    }

    #[test]
    fn cable_partner_matches_the_words_case_insensitively() {
        assert_eq!(cable_partner("voicemeeter input device").as_deref(), Some("voicemeeter Output device"));
    }
}
