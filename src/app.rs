//! Window shell: owns the one `Calculator` and draws display + keypad into it.

use eframe::egui;
use rust_calc::core::Calculator;

pub struct CalcApp {
    calc: Calculator,
    /// Keypad area from the last layout pass, so tests can click a real button.
    keypad_area: egui::Rect,
}

impl Default for CalcApp {
    fn default() -> Self {
        Self {
            calc: Calculator::new(),
            keypad_area: egui::Rect::NOTHING,
        }
    }
}

impl CalcApp {
    /// Everything except the eframe plumbing, so headless tests can run a real egui pass.
    fn show(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            crate::ui::display::show(ui, &self.calc);
            self.keypad_area = crate::ui::keypad::show(ui, &mut self.calc);
        });
    }
}

impl eframe::App for CalcApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::keypad;
    use eframe::egui::{self, Context, Event, Modifiers, Pos2, RawInput, Rect, Vec2};

    const CLIENT: Vec2 = egui::vec2(360.0, 560.0);

    /// One egui pass with one input event. `run_ui` needs no window and no GPU.
    fn pass(app: &mut CalcApp, ctx: &Context, event: Option<Event>) {
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, CLIENT)),
            events: event.into_iter().collect(),
            ..RawInput::default()
        };
        ctx.run_ui(raw, |ui| app.show(ui))
            .drop_without_applying_deltas();
    }

    /// Move, press, release over a key's centre — what a mouse click does.
    fn tap(app: &mut CalcApp, ctx: &Context, label: &str) {
        let pos =
            keypad::key_center(app.keypad_area, label).unwrap_or_else(|| panic!("no key {label}"));
        let button = |pressed: bool| Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        pass(app, ctx, Some(Event::PointerMoved(pos)));
        pass(app, ctx, Some(button(true)));
        pass(app, ctx, Some(button(false)));
    }

    fn startup() -> (CalcApp, Context) {
        let mut app = CalcApp::default();
        let ctx = Context::default();
        pass(&mut app, &ctx, None); // lay out so keypad_area is known
        (app, ctx)
    }

    #[test]
    fn clicking_two_plus_three_equals_shows_five() {
        let (mut app, ctx) = startup();
        for key in ["2", "+", "3", "="] {
            tap(&mut app, &ctx, key);
        }
        assert_eq!(app.calc.display_text(), "5");
        assert_eq!(app.calc.expression_text(), "2 + 3 =");
    }

    #[test]
    fn every_key_is_clickable() {
        for row in keypad::ROWS {
            for (label, expected) in row {
                let (mut app, ctx) = startup();
                tap(&mut app, &ctx, label);
                assert_eq!(
                    app.calc.last_input(),
                    Some(expected),
                    "clicking {label} did not reach the engine"
                );
            }
        }
    }

    #[test]
    fn the_keypad_gets_the_space_below_the_display() {
        let (app, _ctx) = startup();
        let area = app.keypad_area;
        assert!(area.width() > 300.0, "keypad width {area:?}");
        assert!(
            area.height() > 400.0,
            "keypad squashed by the display: {area:?}"
        );
        assert!(area.max.y <= CLIENT.y);
    }

    #[test]
    fn a_long_result_still_reaches_the_engine() {
        let (mut app, ctx) = startup();
        for key in [
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "1", "2", "3", "4", "5", "6",
        ] {
            tap(&mut app, &ctx, key);
        }
        assert_eq!(app.calc.display_text(), "1,234,567,890,123,456");
    }
}
