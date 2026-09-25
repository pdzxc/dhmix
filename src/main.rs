#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1460.0, 900.0]).with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("StreamMix", options, Box::new(|cc| Ok(Box::new(streammix::ui::App::new(cc)))))
}
