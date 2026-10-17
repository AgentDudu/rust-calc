//! History panel: every `=` lands here, newest first, capped at 100 by the engine.
//! Clicking an entry puts its result back on the display.

#[cfg(test)]
use eframe::egui::Pos2;
use eframe::egui::{Align, Button, Layout, Rect, RichText, ScrollArea, Sense, Ui, Vec2};
use rust_calc::core::Calculator;

pub const WIDTH: f32 = 190.0;
const HEADER: f32 = 28.0;
const ROW_H: f32 = 44.0;
const CLEAR_W: f32 = 48.0;
const SENSE: Sense = Sense::CLICK;

/// Centre of entry `index` inside the list area [`show`] returned.
/// `shortcut:` assumes an unscrolled list; give the helper the scroll offset if deeper rows are needed.
#[cfg(test)]
pub fn entry_center(list: Rect, index: usize) -> Pos2 {
    list.min + Vec2::new(list.width() * 0.5, (index as f32 + 0.5) * ROW_H)
}

/// Draws the panel and reports the list area (below the header).
pub fn show(ui: &mut Ui, calc: &mut Calculator) -> Rect {
    let full = ui.max_rect();
    let mut clicked: Option<usize> = None;
    let mut clear = false;

    let header = Rect::from_min_size(full.min, Vec2::new(full.width(), HEADER));
    ui.allocate_ui_with_layout(header.size(), Layout::left_to_right(Align::Center), |ui| {
        ui.label(RichText::new("History").size(14.0).strong());
        let clear_rect = Rect::from_min_size(
            header.max - Vec2::new(CLEAR_W + 2.0, HEADER),
            Vec2::new(CLEAR_W, HEADER - 4.0),
        );
        if ui
            .put(
                clear_rect,
                Button::new(RichText::new("Clear").size(12.0)).sense(SENSE),
            )
            .clicked()
        {
            clear = true;
        }
    });

    let list = Rect::from_min_max(full.min + Vec2::new(0.0, HEADER), full.max);
    // Reuse would overwrite the error, so the list waits until the keypad is live again.
    let locked = calc.has_error();
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if calc.history().is_empty() {
                ui.label(RichText::new("There's nothing here.").weak().size(12.0));
            }
            ui.add_enabled_ui(!locked, |ui| {
                for (index, entry) in calc.history().iter().enumerate() {
                    // The row rect comes back from egui, so it stays right once the list scrolls.
                    let rect = ui
                        .allocate_ui_with_layout(
                            Vec2::new(list.width(), ROW_H),
                            Layout::top_down(Align::Min),
                            |ui| ui.max_rect(),
                        )
                        .inner;
                    let button = Button::new((
                        RichText::new(&entry.expression).weak().size(11.0),
                        RichText::new(&entry.result).strong().size(15.0),
                    ))
                    .min_size(rect.size() - Vec2::new(4.0, 4.0))
                    .sense(SENSE);
                    if ui.put(rect.shrink(2.0), button).clicked() {
                        clicked = Some(index);
                    }
                }
            });
        });

    if clear {
        calc.clear_history();
    }
    if let Some(index) = clicked {
        calc.use_history(index);
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_stack_from_the_top_of_the_list() {
        let list = Rect::from_min_size(Pos2::new(170.0, 28.0), Vec2::new(190.0, 400.0));
        let first = entry_center(list, 0);
        let second = entry_center(list, 1);
        assert!(list.contains(first) && list.contains(second));
        assert_eq!(second.y - first.y, ROW_H);
    }

    #[test]
    fn entries_are_clicked_not_tabbed_into() {
        assert!(!SENSE.is_focusable());
    }
}
