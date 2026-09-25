//! The system tray icon: the "DH" badge with a Show / Quit menu, so closing the window can keep
//! the mixer running. Its events arrive on the main thread from the OS, not from a frame, so
//! they work while the window is hidden.

use crate::app_settings::APP_ID;
use crate::{APP_ICON_RGBA, APP_ICON_SIZE};
use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub struct Tray {
    _icon: TrayIcon,
}

impl Tray {
    /// Puts the icon in the tray. `quit` is set when the user picks Quit, so the close that
    /// follows is not turned into a hide.
    pub fn new(ctx: egui::Context, quit: Arc<AtomicBool>) -> Result<Self> {
        let menu = Menu::new();
        let show = MenuItem::new(format!("Show {APP_ID}"), true, None);
        let quit_item = MenuItem::new(format!("Quit {APP_ID}"), true, None);
        menu.append_items(&[&show, &quit_item]).context("build the tray menu")?;
        let icon = Icon::from_rgba(APP_ICON_RGBA.to_vec(), APP_ICON_SIZE, APP_ICON_SIZE).context("tray icon image")?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(APP_ID)
            .with_icon(icon)
            .with_menu_on_left_click(false)
            .build()
            .context("create the tray icon")?;

        let (show_id, quit_id) = (show.id().clone(), quit_item.id().clone());
        let menu_ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id == show_id {
                show_window(&menu_ctx);
            } else if event.id == quit_id {
                quit.store(true, Ordering::Relaxed);
                menu_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                menu_ctx.request_repaint();
            }
        }));
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_window(&ctx);
            }
        }));
        Ok(Self { _icon: tray })
    }
}

/// Brings the mixer window back from the tray.
pub fn show_window(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.request_repaint();
}
