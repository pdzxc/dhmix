#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dhmix::app_settings::AppSettings;

fn main() -> eframe::Result {
    env_logger::init();
    let app_settings = AppSettings::load();
    let size = [dhmix::ui::WINDOW_SIZE.x, dhmix::ui::WINDOW_SIZE.y];
    let icon = egui::IconData { rgba: dhmix::APP_ICON_RGBA.to_vec(), width: dhmix::APP_ICON_SIZE, height: dhmix::APP_ICON_SIZE };
    let options = eframe::NativeOptions {
        // One fixed size, like a hardware console: five columns share the width and the faders
        // fill the height. No resize, no zoom / full screen, and a previously saved window size
        // must not override it. "Start in tray" begins hidden; the tray icon brings it back.
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
            .with_resizable(false)
            .with_maximize_button(false)
            .with_fullscreen(false)
            .with_visible(!app_settings.start_in_tray)
            .with_icon(icon),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(dhmix::app_settings::APP_ID, options, Box::new(|cc| Ok(Box::new(dhmix::ui::App::new(cc)))))
}
