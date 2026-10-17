//! Window shell: owns the one `Calculator` and draws display + keypad into it.

use eframe::egui;
use rust_calc::core::Calculator;

pub struct CalcApp {
    calc: Calculator,
    /// Last window title sent. Windows puts the result in the title bar for screen readers,
    /// so it is refreshed whenever the display changes and never twice in a row.
    title: String,
    /// Keypad area from the last layout pass, so tests can click a real button.
    keypad_area: egui::Rect,
}

impl Default for CalcApp {
    fn default() -> Self {
        Self {
            calc: Calculator::new(),
            title: String::new(),
            keypad_area: egui::Rect::NOTHING,
        }
    }
}

impl CalcApp {
    /// Everything except the eframe plumbing, so headless tests can run a real egui pass.
    fn show(&mut self, ui: &mut egui::Ui) {
        // Keys first: a keystroke must reach the engine even if a widget has focus.
        crate::ui::keyboard::poll(ui, &mut self.calc);

        let title = format!("{} - Calculator", self.calc.display_text());
        if title != self.title {
            self.title = title;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(self.title.clone()));
        }

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

    /// One egui pass carrying these events. `run_ui` needs no window and no GPU.
    fn pass(app: &mut CalcApp, ctx: &Context, events: impl IntoIterator<Item = Event>) {
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, CLIENT)),
            events: events.into_iter().collect(),
            ..RawInput::default()
        };
        ctx.run_ui(raw, |ui| app.show(ui))
            .drop_without_applying_deltas();
    }

    /// Type a whole string in one pass — the events a fast typist sends.
    fn type_text(app: &mut CalcApp, ctx: &Context, text: &str) {
        pass(app, ctx, [Event::Text(text.into())]);
    }

    fn press(app: &mut CalcApp, ctx: &Context, key: egui::Key) {
        pass(
            app,
            ctx,
            [Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
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
        pass(app, ctx, [Event::PointerMoved(pos)]);
        pass(app, ctx, [button(true)]);
        pass(app, ctx, [button(false)]);
    }

    fn startup() -> (CalcApp, Context) {
        let mut app = CalcApp::default();
        let ctx = Context::default();
        pass(&mut app, &ctx, []); // lay out so keypad_area is known
        (app, ctx)
    }

    #[test]
    fn typing_a_whole_sum_on_the_keyboard_needs_no_mouse() {
        let (mut app, ctx) = startup();
        type_text(&mut app, &ctx, "12+7");
        press(&mut app, &ctx, egui::Key::Enter);
        assert_eq!(app.calc.display_text(), "19");
        assert_eq!(app.calc.expression_text(), "12 + 7 =");
    }

    #[test]
    fn typing_works_right_after_clicking_a_button() {
        let (mut app, ctx) = startup();
        tap(&mut app, &ctx, "6");
        type_text(&mut app, &ctx, "+4=");
        assert_eq!(app.calc.display_text(), "10");
        assert_eq!(ctx.memory(|m| m.focused()), None, "a button stole focus");
    }

    #[test]
    fn equals_fires_once_per_keypress() {
        let (mut app, ctx) = startup();
        type_text(&mut app, &ctx, "2+3=");
        // Mapping Key::Equals as well as the `=` character would repeat equals: 5 -> 8.
        assert_eq!(app.calc.display_text(), "5");
    }

    #[test]
    fn the_title_bar_shows_the_result() {
        let (mut app, ctx) = startup();
        type_text(&mut app, &ctx, "9*9=");
        assert_eq!(app.title, "81 - Calculator");
    }

    #[test]
    fn delete_clears_the_entry_and_keeps_the_pending_step() {
        let (mut app, ctx) = startup();
        type_text(&mut app, &ctx, "12+7");
        press(&mut app, &ctx, egui::Key::Delete);
        assert_eq!(app.calc.display_text(), "0");
        type_text(&mut app, &ctx, "3=");
        assert_eq!(app.calc.display_text(), "15");
    }

    #[test]
    fn escape_clears_the_pending_step_too() {
        let (mut app, ctx) = startup();
        type_text(&mut app, &ctx, "5*");
        press(&mut app, &ctx, egui::Key::Escape);
        type_text(&mut app, &ctx, "3=");
        assert_eq!(app.calc.display_text(), "3");
    }

    #[test]
    fn backspace_and_f9_edit_the_entry() {
        let (mut app, ctx) = startup();
        type_text(&mut app, &ctx, "123");
        press(&mut app, &ctx, egui::Key::Backspace);
        assert_eq!(app.calc.display_text(), "12");
        press(&mut app, &ctx, egui::Key::F9);
        assert_eq!(app.calc.display_text(), "-12");
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
