//! Keyboard input, read once per frame before any widget is drawn.
//!
//! Keys that type a character (`0-9 . , + - * / = %`) arrive as `Event::Text`, which is
//! layout-independent and already handles shifted symbols. The rest have no text form and are
//! matched on `Key::*`. Nothing is mapped on both paths: `=` firing twice would fire equals-repeat.
//!
//! egui-winit 0.37 (checked in `on_key_input`) pushes `Event::Key` for every key and *also*
//! `Event::Text` when the key produced a printable char, with control chars filtered and text
//! suppressed while Ctrl/Command is held. So `5` arrives as `Key::Num5` + `Text("5")` — only the
//! text is mapped, hence one digit — and `Enter` arrives as `Key::Enter` alone.

use eframe::egui::{Event, Key, Ui};
use rust_calc::core::{Calculator, Input};

/// Panel commands — they change the window, not the calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    ToggleHistory,
    ClearHistory,
}

/// Applies key presses to the calculator and returns the commands the shell must handle.
pub fn poll(ui: &mut Ui, calc: &mut Calculator) -> Vec<Command> {
    let mut commands = Vec::new();
    let inputs = ui.input(|i| {
        let mut inputs = Vec::new();
        for event in &i.events {
            inputs.extend(inputs_for(event));
            if let Some(command) = command_for(event) {
                commands.push(command);
            }
        }
        inputs
    });
    for input in inputs {
        calc.input(input);
    }
    commands
}

/// `Ctrl` on Windows/Linux, `Cmd` on macOS — `Modifiers::command` is already the right one.
/// No text event arrives with the modifier held, so these cannot double-fire with `Event::Text`.
fn command_for(event: &Event) -> Option<Command> {
    let Event::Key {
        key,
        pressed: true,
        modifiers,
        ..
    } = event
    else {
        return None;
    };
    if !modifiers.command {
        return None;
    }
    match key {
        Key::H => Some(Command::ToggleHistory),
        Key::D => Some(Command::ClearHistory),
        _ => None,
    }
}

fn inputs_for(event: &Event) -> Vec<Input> {
    match event {
        Event::Text(text) => text.chars().filter_map(Input::from_char).collect(),
        Event::Key {
            key, pressed: true, ..
        } => match key {
            Key::Enter => vec![Input::Equals],
            Key::Backspace => vec![Input::Backspace],
            Key::Delete => vec![Input::ClearEntry],
            Key::Escape => vec![Input::Clear],
            Key::F9 => vec![Input::Negate],
            _ => vec![],
        },
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Event, Key, Modifiers, Pos2};

    fn key(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn typed_characters_become_inputs() {
        for (text, expected) in [
            ("7", Input::Digit('7')),
            (",", Input::Dot),
            ("*", Input::Mul),
            ("%", Input::Percent),
        ] {
            let Event::Text(_) = Event::Text(text.into()) else {
                unreachable!()
            };
            assert_eq!(
                inputs_for(&Event::Text(text.into())),
                vec![expected],
                "typing {text:?}"
            );
        }
    }

    #[test]
    fn enter_equals_backspace_delete_escape_and_f9_are_mapped() {
        assert_eq!(inputs_for(&key(Key::Enter)), vec![Input::Equals]);
        assert_eq!(inputs_for(&key(Key::Backspace)), vec![Input::Backspace]);
        assert_eq!(inputs_for(&key(Key::Delete)), vec![Input::ClearEntry]);
        assert_eq!(inputs_for(&key(Key::Escape)), vec![Input::Clear]);
        assert_eq!(inputs_for(&key(Key::F9)), vec![Input::Negate]);
    }

    #[test]
    fn the_equals_key_is_not_mapped_twice() {
        // `=` arrives as Text *and* as Key::Equals. Firing both would repeat equals.
        assert_eq!(inputs_for(&key(Key::Equals)), Vec::new());
        assert_eq!(inputs_for(&key(Key::Minus)), Vec::new());
    }

    #[test]
    fn unrelated_keys_are_ignored() {
        assert_eq!(inputs_for(&key(Key::Tab)), Vec::new());
        assert_eq!(
            inputs_for(&Event::PointerMoved(Pos2::default())),
            Vec::new()
        );
    }

    fn key_with(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn panel_shortcuts_become_commands() {
        assert_eq!(
            command_for(&key_with(Key::H, Modifiers::COMMAND)),
            Some(Command::ToggleHistory)
        );
        assert_eq!(
            command_for(&key_with(Key::D, Modifiers::COMMAND)),
            Some(Command::ClearHistory)
        );
    }

    #[test]
    fn a_command_needs_the_modifier_and_a_press() {
        assert_eq!(command_for(&key_with(Key::H, Modifiers::NONE)), None);
        assert_eq!(command_for(&key_with(Key::H, Modifiers::ALT)), None);
        let released = Event::Key {
            key: Key::H,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        };
        assert_eq!(command_for(&released), None);
    }

    #[test]
    fn a_shortcut_is_not_also_a_calculator_input() {
        assert_eq!(
            inputs_for(&key_with(Key::H, Modifiers::COMMAND)),
            Vec::new()
        );
        assert_eq!(
            inputs_for(&key_with(Key::D, Modifiers::COMMAND)),
            Vec::new()
        );
    }
}
