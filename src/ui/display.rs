//! The two display lines: dim expression above, big right-aligned result below.

use eframe::egui::{Align, Layout, Rect, RichText, Ui, Vec2};
use rust_calc::core::Calculator;

const MAX_FONT: f32 = 42.0;
const MIN_FONT: f32 = 13.0;
const EXPRESSION_FONT: f32 = 15.0;
/// Rough average glyph width as a fraction of the font size for the default proportional font.
const GLYPH_FACTOR: f32 = 0.58;
/// Line box height as a multiple of the font size.
const LINE_HEIGHT: f32 = 1.35;

pub fn show(ui: &mut Ui, calc: &Calculator) {
    let width = ui.available_width();

    let expression = calc.expression_text();
    if !expression.is_empty() {
        right_aligned(ui, EXPRESSION_FONT * LINE_HEIGHT, |ui| {
            ui.label(RichText::new(expression).weak().size(EXPRESSION_FONT));
        });
    }

    let text = calc.display_text();
    // Shrink to fit instead of clipping: a long result must still be readable.
    let size =
        (width / (text.chars().count().max(1) as f32 * GLYPH_FACTOR)).clamp(MIN_FONT, MAX_FONT);
    right_aligned(ui, size * LINE_HEIGHT, |ui| {
        ui.label(RichText::new(text).size(size));
    });
}

/// `Ui::with_layout` swallows every pixel left in the panel, so the keypad below it would get
/// none. `allocate_ui_with_layout` takes the height we ask for and moves the cursor on.
fn right_aligned(ui: &mut Ui, height: f32, add: impl FnOnce(&mut Ui)) {
    let rect = Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), height));
    ui.allocate_ui_with_layout(rect.size(), Layout::right_to_left(Align::Center), |ui| {
        add(ui);
    });
}
