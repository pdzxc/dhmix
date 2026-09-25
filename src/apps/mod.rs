//! Running applications that play or record audio, and per-app device routing.
//!
//! Windows keeps a per-process default device (the "App volume and device preferences" page).
//! Reading which device an app is on comes from WASAPI audio sessions; changing it uses the same
//! internal policy interface EarTrumpet uses. Other platforms report nothing.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flow {
    /// The app plays sound (game, browser, music player).
    Playback,
    /// The app records sound (Discord, OBS, a recorder).
    Capture,
}

/// One process with at least one audio session.
#[derive(Clone, Debug, PartialEq)]
pub struct AppSession {
    pub pid: u32,
    /// Executable name without extension, or the session's display name.
    pub name: String,
    pub flow: Flow,
    /// Friendly name of the device the session is currently on.
    pub device_name: String,
}

/// An endpoint the user can move an app to.
#[derive(Clone, Debug, PartialEq)]
pub struct Endpoint {
    pub id: String,
    pub name: String,
    pub flow: Flow,
}

#[cfg(windows)]
mod windows_impl;

/// Apps with live audio sessions and the endpoints they can be moved to, from one device pass.
/// Both are empty where per-app routing is unsupported.
pub fn snapshot() -> (Vec<AppSession>, Vec<Endpoint>) {
    #[cfg(windows)]
    {
        windows_impl::snapshot().unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        (Vec::new(), Vec::new())
    }
}

/// Points one process at a device for all its future streams. `None` restores the Windows default.
pub fn set_app_device(pid: u32, flow: Flow, endpoint_id: Option<&str>) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        windows_impl::set_app_device(pid, flow, endpoint_id)
    }
    #[cfg(not(windows))]
    {
        let _ = (pid, flow, endpoint_id);
        anyhow::bail!("Per-app routing is only available on Windows")
    }
}

pub fn supported() -> bool {
    cfg!(windows)
}

/// Processes that should not be offered for routing.
pub fn is_system_process(name: &str) -> bool {
    matches!(name.to_ascii_lowercase().as_str(), "system sounds" | "audiodg" | "explorer" | "streammix" | "shellexperiencehost")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_processes_are_filtered_out() {
        assert!(is_system_process("AudioDg"));
        assert!(is_system_process("streammix"));
        assert!(!is_system_process("Discord"));
    }

    #[test]
    fn system_process_matching_is_case_insensitive_for_every_entry() {
        assert!(is_system_process("SYSTEM SOUNDS"));
        assert!(is_system_process("AUDIODG"));
        assert!(is_system_process("Explorer"));
        assert!(is_system_process("StreamMix"));
        assert!(is_system_process("ShellExperienceHost"));
        assert!(!is_system_process("DISCORD"));
    }

    #[cfg(not(windows))]
    #[test]
    fn unsupported_platforms_report_nothing_and_refuse_to_route() {
        assert!(snapshot().0.is_empty());
        assert!(!supported());
        assert!(set_app_device(1, Flow::Playback, None).is_err());
    }
}
