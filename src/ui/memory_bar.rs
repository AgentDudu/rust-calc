//! Memory bar: `MC MR M+ M- MS` and the `(M)` indicator Windows shows while memory holds a value.

#[cfg(test)]
use eframe::egui::Pos2;
use eframe::egui::{Align, Button, Label, Layout, Rect, RichText, Sense, Ui, Vec2};
use rust_calc::core::Calculator;

pub const BUTTONS: [&str; 5] = ["MC", "MR", "M+", "M-", "MS"];
/// MC, MR and M- are dead until something is stored. M+ and MS are not: on an empty memory,
/// M+ stores what is on the display.
const NEEDS_MEMORY: [bool; 5] = [true, true, false, true, false];

const HEIGHT: f32 = 28.0;
const BUTTON_W: f32 = 44.0;
const GAP: f32 = 2.0;
/// Click without focus — see `keypad::SENSE`.
const SENSE: Sense = Sense::CLICK;

/// Centre of the memory button with this label, in the area [`show`] returned.
#[cfg(test)]
pub fn button_center(area: Rect, label: &str) -> Option<Pos2> {
    let index = BUTTONS.iter().position(|name| *name == label)?;
    Some(area.min + Vec2::new((index as f32 + 0.5) * (BUTTON_W + GAP), HEIGHT * 0.5))
}

/// Draws the bar and reports the area it used, so a headless test can click a real button.
pub fn show(ui: &mut Ui, calc: &mut Calculator) -> Rect {
    let area = Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), HEIGHT));
    let has_memory = calc.has_memory();
    // Every memory op except MC reads the display, and during an error the display is a sentence.
    let locked = calc.has_error();

    // The child ui only reserves the strip; the widgets are placed at explicit rects.
    ui.allocate_ui_with_layout(area.size(), Layout::top_down(Align::Min), |ui| {
        for (index, label) in BUTTONS.iter().enumerate() {
            let rect = Rect::from_min_size(
                area.min + Vec2::new(index as f32 * (BUTTON_W + GAP), 0.0),
                Vec2::new(BUTTON_W, HEIGHT),
            );
            let button = Button::new(RichText::new(*label).size(13.0))
                .min_size(rect.size())
                .sense(SENSE);
            let enabled = !locked || *label == "MC";
            let enabled = enabled && (has_memory || !NEEDS_MEMORY[index]);
            let clicked = ui
                .add_enabled_ui(enabled, |ui| ui.put(rect, button))
                .inner
                .clicked();
            if clicked {
                match *label {
                    "MC" => calc.memory_clear(),
                    "MR" => calc.memory_recall(),
                    "M+" => calc.memory_add(),
                    "M-" => calc.memory_subtract(),
                    _ => calc.memory_store(),
                }
            }
        }

        if has_memory {
            let x = BUTTONS.len() as f32 * (BUTTON_W + GAP) + 6.0;
            let rect = Rect::from_min_size(
                area.min + Vec2::new(x, 0.0),
                Vec2::new((area.width() - x).max(0.0), HEIGHT),
            );
            ui.put(rect, Label::new(RichText::new("(M)").weak().size(13.0)));
        }
    });

    area
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_button_lands_inside_the_bar() {
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(360.0, HEIGHT));
        for label in BUTTONS {
            let pos = button_center(area, label).unwrap();
            assert!(area.contains(pos), "{label} outside the bar");
        }
    }

    #[test]
    fn buttons_do_not_take_keyboard_focus() {
        assert!(!SENSE.is_focusable(), "a focusable button swallows Enter");
    }
}
