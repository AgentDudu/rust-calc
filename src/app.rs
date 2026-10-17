//! Window shell: owns the one `Calculator` and draws display + keypad into it.

use eframe::egui;
use rust_calc::core::Calculator;

use crate::ui::keyboard::Command;

pub struct CalcApp {
    calc: Calculator,
    /// Last window title sent. Windows puts the result in the title bar for screen readers,
    /// so it is refreshed whenever the display changes and never twice in a row.
    title: String,
    history_open: bool,
    /// Areas from the last layout pass, so tests can click a real widget.
    keypad_area: egui::Rect,
    memory_area: egui::Rect,
    history_area: egui::Rect,
    history_toggle: egui::Rect,
}

impl Default for CalcApp {
    fn default() -> Self {
        Self {
            calc: Calculator::new(),
            title: String::new(),
            history_open: false,
            keypad_area: egui::Rect::NOTHING,
            memory_area: egui::Rect::NOTHING,
            history_area: egui::Rect::NOTHING,
            history_toggle: egui::Rect::NOTHING,
        }
    }
}

impl CalcApp {
    /// Everything except the eframe plumbing, so headless tests can run a real egui pass.
    fn show(&mut self, ui: &mut egui::Ui) {
        // Keys first: a keystroke must reach the engine even if a widget has focus.
        for command in crate::ui::keyboard::poll(ui, &mut self.calc) {
            match command {
                Command::ToggleHistory => self.history_open = !self.history_open,
                Command::ClearHistory => self.calc.clear_history(),
            }
        }

        // Side panel before the others, so the bars above the keypad stop at its edge.
        if self.history_open {
            let panel = egui::Panel::right("history_panel")
                .exact_size(crate::ui::history::WIDTH)
                .resizable(false)
                .show(ui, |ui| crate::ui::history::show(ui, &mut self.calc));
            self.history_area = panel.inner;
        }

        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                let toggle = ui.selectable_label(self.history_open, "History");
                self.history_toggle = toggle.rect;
                if toggle.clicked() {
                    self.history_open = !self.history_open;
                }
            });
            self.memory_area = crate::ui::memory_bar::show(ui, &mut self.calc);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            crate::ui::display::show(ui, &self.calc);
            self.keypad_area = crate::ui::keypad::show(ui, &mut self.calc);
        });

        // After the widgets: a click is applied while they draw, so a title computed before them
        // would show the previous frame's value and, with no further input, stay there.
        let title = format!("{} - Calculator", self.calc.display_text());
        if title != self.title {
            self.title = title;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(self.title.clone()));
        }
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
    use crate::ui::{history, keypad, memory_bar};
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
        pass(app, ctx, [key_event(key, Modifiers::NONE)]);
    }

    fn press_with(app: &mut CalcApp, ctx: &Context, key: egui::Key, modifiers: Modifiers) {
        pass(app, ctx, [key_event(key, modifiers)]);
    }

    fn key_event(key: egui::Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    /// Move, press, release over a point — what a mouse click does.
    fn tap_pos(app: &mut CalcApp, ctx: &Context, pos: Pos2) {
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

    fn tap(app: &mut CalcApp, ctx: &Context, label: &str) {
        let pos =
            keypad::key_center(app.keypad_area, label).unwrap_or_else(|| panic!("no key {label}"));
        tap_pos(app, ctx, pos);
    }

    fn tap_memory(app: &mut CalcApp, ctx: &Context, label: &str) {
        let pos = memory_bar::button_center(app.memory_area, label)
            .unwrap_or_else(|| panic!("no memory button {label}"));
        tap_pos(app, ctx, pos);
    }

    fn tap_history_entry(app: &mut CalcApp, ctx: &Context, index: usize) {
        tap_pos(app, ctx, history::entry_center(app.history_area, index));
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

    #[test]
    fn memory_store_add_and_recall_with_the_mouse() {
        let (mut app, ctx) = startup();
        type_text(&mut app, &ctx, "7");
        tap_memory(&mut app, &ctx, "MS");
        assert!(app.calc.has_memory(), "MS left the memory empty");

        // MS leaves the entry alone, so CE comes first — same as Windows.
        tap(&mut app, &ctx, "CE");
        type_text(&mut app, &ctx, "5");
        tap_memory(&mut app, &ctx, "M+");
        assert_eq!(app.calc.memory_value(), 12.0);

        tap(&mut app, &ctx, "CE");
        tap_memory(&mut app, &ctx, "MR");
        assert_eq!(app.calc.display_text(), "12");

        tap_memory(&mut app, &ctx, "MC");
        assert!(!app.calc.has_memory());
    }

    #[test]
    fn m_minus_subtracts_the_display_from_memory() {
        let (mut app, ctx) = startup();
        type_text(&mut app, &ctx, "9");
        tap_memory(&mut app, &ctx, "MS");
        tap(&mut app, &ctx, "CE");
        type_text(&mut app, &ctx, "4");
        tap_memory(&mut app, &ctx, "M-");
        assert_eq!(app.calc.memory_value(), 5.0);
    }

    #[test]
    fn memory_buttons_are_dead_until_something_is_stored() {
        let (mut app, ctx) = startup();
        tap_memory(&mut app, &ctx, "MR");
        tap_memory(&mut app, &ctx, "MC");
        assert_eq!(app.calc.display_text(), "0");
        assert!(!app.calc.has_memory());

        // M+ is not dead: on an empty memory it stores, like Windows.
        type_text(&mut app, &ctx, "3");
        tap_memory(&mut app, &ctx, "M+");
        assert_eq!(app.calc.memory_value(), 3.0);
    }

    #[test]
    fn ctrl_h_opens_and_closes_the_history_panel() {
        let (mut app, ctx) = startup();
        press_with(&mut app, &ctx, egui::Key::H, Modifiers::COMMAND);
        assert!(app.history_open);
        assert!(
            app.history_area.height() > 100.0,
            "panel not laid out: {:?}",
            app.history_area
        );

        press_with(&mut app, &ctx, egui::Key::H, Modifiers::COMMAND);
        assert!(!app.history_open);
    }

    #[test]
    fn the_history_button_toggles_the_panel_without_a_keyboard() {
        let (mut app, ctx) = startup();
        let toggle = app.history_toggle.center();
        tap_pos(&mut app, &ctx, toggle);
        assert!(app.history_open, "the History button did nothing");
        tap_pos(&mut app, &ctx, toggle);
        assert!(!app.history_open);
    }

    #[test]
    fn ctrl_d_clears_the_history() {
        let (mut app, ctx) = startup();
        type_text(&mut app, &ctx, "2+3=");
        assert_eq!(app.calc.history().len(), 1);
        press_with(&mut app, &ctx, egui::Key::D, Modifiers::COMMAND);
        assert!(app.calc.history().is_empty());
    }

    #[test]
    fn clicking_a_history_entry_puts_its_result_back_on_the_display() {
        let (mut app, ctx) = startup();
        type_text(&mut app, &ctx, "2+3=");
        type_text(&mut app, &ctx, "999");
        press_with(&mut app, &ctx, egui::Key::H, Modifiers::COMMAND);

        tap_history_entry(&mut app, &ctx, 0);
        assert_eq!(app.calc.display_text(), "5");

        // A reused result behaves like a fresh entry: the next digit replaces it.
        type_text(&mut app, &ctx, "7");
        assert_eq!(app.calc.display_text(), "7");
    }

    #[test]
    fn the_bars_sit_above_the_keypad_and_inside_the_window() {
        let (app, _ctx) = startup();
        let client = Rect::from_min_size(Pos2::ZERO, CLIENT);
        assert!(
            client.contains_rect(app.memory_area) && client.contains_rect(app.history_toggle),
            "top bar outside the window: {:?} {:?}",
            app.memory_area,
            app.history_toggle
        );
        assert!(
            app.memory_area.max.y <= app.keypad_area.min.y + 1.0,
            "memory bar overlaps the keypad: {:?} vs {:?}",
            app.memory_area,
            app.keypad_area
        );
    }

    #[test]
    fn the_history_panel_shrinks_the_keypad_instead_of_covering_it() {
        let (mut app, ctx) = startup();
        let before = app.keypad_area.width();
        press_with(&mut app, &ctx, egui::Key::H, Modifiers::COMMAND);
        assert!(
            app.keypad_area.width() < before - history::WIDTH + 20.0,
            "keypad {:?} with the panel open",
            app.keypad_area
        );
        assert!(app.keypad_area.max.x <= app.history_area.min.x + 1.0);
    }

    #[test]
    fn divide_by_zero_shows_the_message_and_locks_the_keypad() {
        let (mut app, ctx) = startup();
        for key in ["1", "÷", "0", "="] {
            tap(&mut app, &ctx, key);
        }
        assert_eq!(app.calc.display_text(), "Cannot divide by zero");
        assert_eq!(app.title, "Cannot divide by zero - Calculator");
        assert_eq!(app.calc.expression_text(), "1 ÷ 0 =");

        for key in ["7", "+", "×"] {
            tap(&mut app, &ctx, key);
        }
        assert_eq!(
            app.calc.display_text(),
            "Cannot divide by zero",
            "a greyed key woke up"
        );

        tap(&mut app, &ctx, "CE");
        assert_eq!(app.calc.display_text(), "0");
        tap(&mut app, &ctx, "7");
        assert_eq!(app.calc.display_text(), "7");
    }

    #[test]
    fn c_clears_the_error_and_starts_over() {
        let (mut app, ctx) = startup();
        for key in ["5", "÷", "0", "="] {
            tap(&mut app, &ctx, key);
        }
        assert!(app.calc.has_error());
        tap(&mut app, &ctx, "C");
        assert!(!app.calc.has_error());
        assert_eq!(app.calc.expression_text(), "");
        tap(&mut app, &ctx, "3");
        assert_eq!(app.calc.display_text(), "3");
    }

    #[test]
    fn an_error_freezes_the_memory_bar_except_mc() {
        let (mut app, ctx) = startup();
        type_text(&mut app, &ctx, "5");
        tap_memory(&mut app, &ctx, "MS");
        for key in ["1", "÷", "0", "="] {
            tap(&mut app, &ctx, key);
        }

        tap_memory(&mut app, &ctx, "M+");
        tap_memory(&mut app, &ctx, "MR");
        assert_eq!(app.calc.memory_value(), 5.0, "M+ read an error as a number");
        assert!(app.calc.has_error(), "a memory button cleared the error");

        tap_memory(&mut app, &ctx, "MC");
        assert!(
            !app.calc.has_memory(),
            "MC is the one button that stays alive"
        );
    }

    #[test]
    fn history_cannot_be_reused_over_an_error() {
        let (mut app, ctx) = startup();
        type_text(&mut app, &ctx, "2+3=");
        press_with(&mut app, &ctx, egui::Key::H, Modifiers::COMMAND);
        for key in ["1", "÷", "0", "="] {
            tap(&mut app, &ctx, key);
        }

        tap_history_entry(&mut app, &ctx, 0);
        assert!(app.calc.has_error(), "reuse went through the lockout");

        tap(&mut app, &ctx, "C");
        tap_history_entry(&mut app, &ctx, 0);
        assert_eq!(app.calc.display_text(), "5");
    }

    #[test]
    fn sqrt_of_a_negative_number_is_invalid_input() {
        let (mut app, ctx) = startup();
        for key in ["9", "±", "√x"] {
            tap(&mut app, &ctx, key);
        }
        assert_eq!(app.calc.display_text(), "Invalid input");
        tap(&mut app, &ctx, "C");
        assert_eq!(app.calc.display_text(), "0");
    }

    #[test]
    fn squaring_past_the_range_overflows() {
        let (mut app, ctx) = startup();
        tap(&mut app, &ctx, "9");
        for _ in 0..5 {
            tap(&mut app, &ctx, "x\u{b2}");
        }
        assert_eq!(app.calc.display_text(), "Overflow");
    }

    #[test]
    fn the_keypad_stops_at_16_digits() {
        let (mut app, ctx) = startup();
        for _ in 0..20 {
            tap(&mut app, &ctx, "7");
        }
        assert_eq!(app.calc.display_text().matches('7').count(), 16);
        assert!(!app.calc.has_error(), "a full display is not an error");
    }
}
