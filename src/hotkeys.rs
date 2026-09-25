//! Global hotkeys that work while a game or OBS has focus: Ctrl+Alt+M toggles mute on the chosen
//! strip, Ctrl+Alt+1..9 fire soundboard pads. Presses are delivered to a handler on the main
//! thread by the OS, so they keep working while the window is hidden in the tray.

use anyhow::Result;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    ToggleMute,
    Pad(usize),
}

pub struct Hotkeys {
    _manager: GlobalHotKeyManager,
}

const PAD_CODES: [Code; 9] = [
    Code::Digit1,
    Code::Digit2,
    Code::Digit3,
    Code::Digit4,
    Code::Digit5,
    Code::Digit6,
    Code::Digit7,
    Code::Digit8,
    Code::Digit9,
];

/// Which action a registered hotkey id stands for.
fn action_for(id: u32, mute_id: u32, pad_ids: &[u32]) -> Option<HotkeyAction> {
    if id == mute_id {
        Some(HotkeyAction::ToggleMute)
    } else {
        pad_ids.iter().position(|pad| *pad == id).map(HotkeyAction::Pad)
    }
}

impl Hotkeys {
    /// Registers the keys and calls `on_action` for every press, from the OS event thread.
    pub fn register(on_action: impl Fn(HotkeyAction) + Send + Sync + 'static) -> Result<Self> {
        let manager = GlobalHotKeyManager::new()?;
        let mods = Modifiers::CONTROL | Modifiers::ALT;
        let mute = HotKey::new(Some(mods), Code::KeyM);
        manager.register(mute)?;
        let mut pad_ids = Vec::new();
        for code in PAD_CODES {
            let hk = HotKey::new(Some(mods), code);
            manager.register(hk)?;
            pad_ids.push(hk.id());
        }
        let mute_id = mute.id();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state == HotKeyState::Pressed {
                if let Some(action) = action_for(event.id, mute_id, &pad_ids) {
                    on_action(action);
                }
            }
        }));
        Ok(Self { _manager: manager })
    }

    pub fn description() -> &'static str {
        "Ctrl+Alt+M mute · Ctrl+Alt+1-9 pads"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_ids_map_to_mute_and_pads_and_nothing_else() {
        let pads = [10, 11, 12];
        assert_eq!(action_for(7, 7, &pads), Some(HotkeyAction::ToggleMute));
        assert_eq!(action_for(12, 7, &pads), Some(HotkeyAction::Pad(2)));
        assert_eq!(action_for(99, 7, &pads), None);
    }
}
