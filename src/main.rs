#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod ui;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Calculator")
            .with_inner_size([360.0, 560.0])
            .with_min_inner_size([320.0, 420.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Calculator",
        options,
        Box::new(|_cc| Ok(Box::new(app::CalcApp::default()))),
    )
}
