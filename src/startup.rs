//! "Run on startup": registers the executable as a login item with the OS (a Run registry entry
//! on Windows, a launch agent on macOS) through the `auto-launch` crate.

use crate::app_settings::APP_ID;
use anyhow::{Context, Result};
use auto_launch::{AutoLaunch, AutoLaunchBuilder, MacOSLaunchMode};

fn login_item() -> Result<AutoLaunch> {
    let exe = std::env::current_exe().context("locate the executable")?;
    AutoLaunchBuilder::new()
        .set_app_name(APP_ID)
        .set_app_path(&exe.to_string_lossy())
        .set_macos_launch_mode(MacOSLaunchMode::LaunchAgent)
        .build()
        .context("prepare the login item")
}

pub fn set_run_on_startup(enable: bool) -> Result<()> {
    let item = login_item()?;
    if enable {
        item.enable().context("register DHMIX to run on startup")
    } else {
        item.disable().context("remove DHMIX from startup")
    }
}

/// What the OS currently says; false when it cannot be read.
pub fn runs_on_startup() -> bool {
    login_item().and_then(|item| item.is_enabled().context("read the login item")).unwrap_or(false)
}
