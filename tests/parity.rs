//! Table-driven parity against the real Windows Calculator (Standard mode).
//!
//! Script alphabet: digits, `. + - * / = %` plus `c` clear, `e` clear entry, `b` backspace,
//! `n` negate, `i` 1/x, `q` x², `r` √x.

use rust_calc::core::{Calculator, Input};

fn key(c: char) -> Input {
    if let Some(input) = Input::from_char(c) {
        return input;
    }
    match c {
        'c' => Input::Clear,
        'e' => Input::ClearEntry,
        'b' => Input::Backspace,
        'n' => Input::Negate,
        'i' => Input::Inv,
        'q' => Input::Sqr,
        'r' => Input::Sqrt,
        other => panic!("unknown script character: {other}"),
    }
}

fn run(script: &str) -> String {
    let mut calc = Calculator::new();
    for c in script.chars() {
        calc.input(key(c));
    }
    // The table marks failures with `Error: `; the engine itself shows Windows' own wording.
    if calc.has_error() {
        format!("Error: {}", calc.display_text())
    } else {
        calc.display_text()
    }
}

const CASES: &[(&str, &str)] = &[
    // arithmetic
    ("7+2=", "9"),
    ("7-2=", "5"),
    ("7*2=", "14"),
    ("7/2=", "3.5"),
    ("7/0=", "Error: Cannot divide by zero"),
    // chaining: the pending step evaluates when the next operator arrives
    ("2+3+4=", "9"),
    ("10-2-3=", "5"),
    ("100/5/2=", "10"),
    // a second operator replaces the first without evaluating
    ("2+*3=", "6"),
    ("10-/2=", "5"),
    // `=` straight after an operator reuses the displayed value as the right operand (Windows quirk)
    ("2+=", "4"),
    ("5*=", "25"),
    ("2+3*=", "25"),
    // equals repeats the last step
    ("2+3==", "8"),
    ("2+3===", "11"),
    ("10-2==", "6"),
    ("2*3==", "18"),
    ("16/2==", "4"),
    ("5=", "5"),
    ("5==", "5"),
    // an operator after a result continues from that result
    ("2+3=*4=", "20"),
    ("2+3=/5=", "1"),
    ("5+5=+5=", "15"),
    ("2+3=+=", "10"),
    // percent is context sensitive
    ("50+10%", "55"),
    ("50-10%", "45"),
    ("50*10%", "5"),
    ("50/10%", "500"),
    ("50%", "0.5"),
    ("200+50%=", "300"),
    // unary keys act on the display immediately
    ("9r", "3"),
    ("7q", "49"),
    ("4i", "0.25"),
    ("2n", "-2"),
    ("0i", "Error: Cannot divide by zero"),
    ("4nr", "Error: Invalid input"),
    // f64 rounding is hidden at 16 significant digits
    ("0.1+0.2=", "0.3"),
    ("1/3=", "0.3333333333333333"),
    ("2/3=", "0.6666666666666666"),
    // thousands separators
    ("12345+6789=", "19,134"),
    ("1000000*1000=", "1,000,000,000"),
    // overflow at 1e16
    ("9999999999999999+1=", "Error: Overflow"),
    ("9999999999999999*99=", "Error: Overflow"),
    // entry is capped at 16 digits
    ("1234567890123456789", "1,234,567,890,123,456"),
    // CE keeps the pending step, C clears everything but memory and history
    ("10+7e=", "10"),
    ("9c5=", "5"),
    ("1/0=c5=", "5"),
    // backspace
    ("123b", "12"),
    ("123bb", "1"),
    ("123bbb", "0"),
    // negate flips the entry, not the accumulator
    ("5-3n=", "8"),
    ("0n", "0"),
];

#[test]
fn display_matches_windows_calculator() {
    let mut failures = Vec::new();
    for (script, expected) in CASES {
        let got = run(script);
        if got != *expected {
            failures.push(format!("{script}: expected {expected}, got {got}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases failed:\n{}",
        failures.len(),
        CASES.len(),
        failures.join("\n")
    );
}

#[test]
fn error_locks_keypad_until_clear() {
    let mut calc = Calculator::new();
    for c in "1/0=".chars() {
        calc.input(key(c));
    }
    calc.input(Input::Digit('7'));
    calc.input(Input::Add);
    assert_eq!(calc.display_text(), "Cannot divide by zero");
    calc.input(Input::Clear);
    assert_eq!(calc.display_text(), "0");
    calc.input(Input::Digit('2'));
    assert_eq!(calc.display_text(), "2");
}

#[test]
fn memory_survives_clear_and_history_survives_too() {
    let mut calc = Calculator::new();
    for c in "12".chars() {
        calc.input(key(c));
    }
    calc.memory_store();
    calc.input(Input::Clear);
    assert_eq!(calc.memory_value(), 12.0);

    for c in "3+4=".chars() {
        calc.input(key(c));
    }
    calc.memory_add();
    assert_eq!(calc.memory_value(), 19.0);
    calc.memory_subtract();
    assert_eq!(calc.memory_value(), 12.0);

    calc.input(Input::Clear);
    calc.memory_recall();
    assert_eq!(calc.display_text(), "12");
    assert_eq!(calc.history().len(), 1);
    calc.memory_clear();
    assert_eq!(calc.memory_value(), 0.0);
}

#[test]
fn history_records_expressions_newest_first() {
    let mut calc = Calculator::new();
    for c in "12+7==".chars() {
        calc.input(key(c));
    }
    let history = calc.history();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].expression, "19 + 7 =");
    assert_eq!(history[0].result, "26");
    assert_eq!(history[1].expression, "12 + 7 =");
    assert_eq!(history[1].result, "19");
}
