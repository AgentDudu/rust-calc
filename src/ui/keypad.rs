//! The 4×6 Standard keypad. Buttons fill the space left by the display lines, so the layout
//! survives window resizing — `egui::Grid` would not stretch.

use eframe::egui::{Button, Rect, RichText, Sense, Ui, Vec2};
use rust_calc::core::{Calculator, Input};

const COLS: usize = 4;
const GAP: f32 = 2.0;

/// Click without focus. `Sense::click()` is focusable, and a focused button swallows Enter —
/// the keyboard belongs to the calculator, not to whichever button was last clicked.
const SENSE: Sense = Sense::CLICK;

pub const ROWS: [[(&str, Input); COLS]; 6] = [
    [
        ("%", Input::Percent),
        ("CE", Input::ClearEntry),
        ("C", Input::Clear),
        ("⌫", Input::Backspace),
    ],
    [
        ("1/x", Input::Inv),
        ("x²", Input::Sqr),
        ("√x", Input::Sqrt),
        ("÷", Input::Div),
    ],
    [
        ("7", Input::Digit('7')),
        ("8", Input::Digit('8')),
        ("9", Input::Digit('9')),
        ("×", Input::Mul),
    ],
    [
        ("4", Input::Digit('4')),
        ("5", Input::Digit('5')),
        ("6", Input::Digit('6')),
        ("−", Input::Sub),
    ],
    [
        ("1", Input::Digit('1')),
        ("2", Input::Digit('2')),
        ("3", Input::Digit('3')),
        ("+", Input::Add),
    ],
    [
        ("±", Input::Negate),
        ("0", Input::Digit('0')),
        (".", Input::Dot),
        ("=", Input::Equals),
    ],
];

/// Centre of the key with this label, in the same coordinates [`show`] was given. Lets a
/// headless test click a real button without duplicating the layout maths.
#[cfg(test)]
pub fn key_center(area: Rect, label: &str) -> Option<eframe::egui::Pos2> {
    let (row, col) = ROWS.iter().enumerate().find_map(|(row, keys)| {
        keys.iter()
            .position(|(key, _)| *key == label)
            .map(|col| (row, col))
    })?;
    let cell = Vec2::new(
        area.width() / COLS as f32,
        area.height() / ROWS.len() as f32,
    );
    Some(area.min + Vec2::new((col as f32 + 0.5) * cell.x, (row as f32 + 0.5) * cell.y))
}

/// Draws the keypad into the space `ui` has left and reports the area it used.
pub fn show(ui: &mut Ui, calc: &mut Calculator) -> Rect {
    let area = ui.available_rect_before_wrap();
    let cell = Vec2::new(
        area.width() / COLS as f32,
        area.height() / ROWS.len() as f32,
    );
    let font_size = (cell.y * 0.34).clamp(12.0, 26.0);

    for (row, keys) in ROWS.iter().enumerate() {
        for (col, (label, input)) in keys.iter().enumerate() {
            let rect = Rect::from_min_size(
                area.min + Vec2::new(col as f32 * cell.x, row as f32 * cell.y),
                cell,
            )
            .shrink(GAP);

            let button = Button::new(RichText::new(*label).size(font_size))
                .min_size(rect.size())
                .sense(SENSE);
            if ui.put(rect, button).clicked() {
                calc.input(*input);
            }
        }
    }
    area
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every state the engine accepts must be reachable from a key, or it is dead code.
    const ALL_INPUTS: [Input; 15] = [
        Input::Digit('1'),
        Input::Dot,
        Input::Add,
        Input::Sub,
        Input::Mul,
        Input::Div,
        Input::Percent,
        Input::Inv,
        Input::Sqr,
        Input::Sqrt,
        Input::Negate,
        Input::Equals,
        Input::Clear,
        Input::ClearEntry,
        Input::Backspace,
    ];

    #[test]
    fn every_engine_input_has_a_key() {
        let keys: Vec<Input> = ROWS.iter().flatten().map(|(_, input)| *input).collect();
        for input in ALL_INPUTS {
            assert!(keys.contains(&input), "no key sends {input:?}");
        }
    }

    #[test]
    fn buttons_do_not_take_keyboard_focus() {
        assert!(!SENSE.is_focusable(), "a focusable button swallows Enter");
    }

    #[test]
    fn labels_are_unique_and_non_empty() {
        let labels: Vec<&str> = ROWS.iter().flatten().map(|(label, _)| *label).collect();
        assert_eq!(labels.len(), 24);
        assert!(labels.iter().all(|label| !label.is_empty()));

        let mut sorted = labels.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), labels.len(), "duplicate key labels");
    }
}
