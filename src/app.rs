use eframe::egui;

/// Shell of the calculator UI. State machine lives in `core` (added with the keypad).
#[derive(Default)]
pub struct CalcApp;

impl eframe::App for CalcApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |_ui| {});
    }
}
