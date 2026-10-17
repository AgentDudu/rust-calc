#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Layout maths turns grid indices into pixel offsets. Those values are keypad-sized — single
// digits to a few hundred — so the usize→f32 widening clippy worries about cannot happen.
#[allow(
    clippy::cast_precision_loss,
    reason = "small layout indices, not usize-sized values"
)]
mod app;
#[allow(
    clippy::cast_precision_loss,
    reason = "small layout indices, not usize-sized values"
)]
mod ui;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Calculator")
            .with_inner_size([360.0, 560.0])
            .with_min_inner_size([320.0, 420.0])
            .with_icon(ui::theme::icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Calculator",
        options,
        Box::new(|cc| {
            ui::theme::install(&cc.egui_ctx);
            Ok(Box::new(app::CalcApp::default()))
        }),
    )
}
