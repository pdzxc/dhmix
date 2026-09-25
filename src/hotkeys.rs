//! Global hotkeys that work while a game or OBS has focus:
//! Ctrl+Alt+M toggles mute on the chosen strip, Ctrl+Alt+1..9 fire soundboard pads.

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
    mute_id: u32,
    pad_ids: Vec<u32>,
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

impl Hotkeys {
    pub fn register() -> Result<Self> {
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
        Ok(Self { _manager: manager, mute_id: mute.id(), pad_ids })
    }

    pub fn description() -> &'static str {
        "Ctrl+Alt+M mute · Ctrl+Alt+1-9 pads"
    }

    pub fn poll(&self) -> Vec<HotkeyAction> {
        let mut actions = Vec::new();
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state != HotKeyState::Pressed {
                continue;
            }
            if event.id == self.mute_id {
                actions.push(HotkeyAction::ToggleMute);
            } else if let Some(pad) = self.pad_ids.iter().position(|id| *id == event.id) {
                actions.push(HotkeyAction::Pad(pad));
            }
        }
        actions
    }
}
