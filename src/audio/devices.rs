//! Endpoint enumeration. On Windows this is WASAPI, so virtual cables (VB-CABLE "CABLE Input" /
//! "CABLE Output") show up here like any other device.

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
    ["cable", "virtual", "vb-audio", "voicemeeter", "blackhole", "loopback"].iter().any(|k| n.contains(k))
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
        assert!(is_virtual_name("Voicemeeter Out B1"));
        assert!(!is_virtual_name("Realtek High Definition Audio"));
    }
}
