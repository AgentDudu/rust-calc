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

pub fn poll(ui: &mut Ui, calc: &mut Calculator) {
    let inputs = ui.input(|i| i.events.iter().flat_map(inputs_for).collect::<Vec<Input>>());
    for input in inputs {
        calc.input(input);
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
    use eframe::egui::{Event, Key, Modifiers};

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
        assert!(inputs_for(&key(Key::Equals)).is_empty());
        assert_eq!(inputs_for(&key(Key::Minus)), Vec::new());
    }

    #[test]
    fn unrelated_keys_are_ignored() {
        assert!(inputs_for(&key(Key::Tab)).is_empty());
        assert!(inputs_for(&Event::PointerMoved(Default::default())).is_empty());
    }
}
