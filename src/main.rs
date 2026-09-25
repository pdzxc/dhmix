#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        // Wide enough for all seven input columns without horizontal scrolling.
        // The layout is responsive: columns share the width and faders stretch into the height.
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 900.0])
            .with_min_inner_size([streammix::ui::MIN_WINDOW.x, streammix::ui::MIN_WINDOW.y]),
        ..Default::default()
    };
    eframe::run_native("DHMIX", options, Box::new(|cc| Ok(Box::new(streammix::ui::App::new(cc)))))
}
