//! The Standard-mode state machine. Every rule in PLAN §4 lands here, not in the UI.

use crate::core::error::CalcError;
use crate::core::eval::{self, Op};
use crate::core::fmt;

/// Windows stops accepting digits at 16.
pub const MAX_INPUT_DIGITS: usize = 16;
const HISTORY_CAP: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Digit(char),
    Dot,
    Add,
    Sub,
    Mul,
    Div,
    Percent,
    Inv,
    Sqr,
    Sqrt,
    Negate,
    Equals,
    Clear,
    ClearEntry,
    Backspace,
}

impl Input {
    /// Keyboard/button character, for the ones that have a key. Phase 3 wires this to input events.
    pub fn from_char(c: char) -> Option<Input> {
        Some(match c {
            '0'..='9' => Input::Digit(c),
            '.' | ',' => Input::Dot,
            '+' => Input::Add,
            '-' => Input::Sub,
            '*' => Input::Mul,
            '/' => Input::Div,
            '=' => Input::Equals,
            '%' => Input::Percent,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub expression: String,
    pub result: String,
}

#[derive(Debug, Clone)]
pub struct Calculator {
    /// Raw input buffer — digits, optional `-`, optional `.`. Never contains separators.
    display: String,
    acc: Option<f64>,
    pending: Option<Op>,
    /// Leftover `(op, operand)` so a second `=` repeats the last step.
    last_op: Option<(Op, f64)>,
    /// Next digit replaces the display instead of appending to it.
    fresh: bool,
    memory: f64,
    history: Vec<Entry>,
    error: Option<CalcError>,
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

impl Calculator {
    pub fn new() -> Self {
        Calculator {
            display: "0".to_string(),
            acc: None,
            pending: None,
            last_op: None,
            fresh: true,
            memory: 0.0,
            history: Vec::new(),
            error: None,
        }
    }

    pub fn input(&mut self, input: Input) {
        use Input::*;

        match input {
            Clear => {
                *self = Calculator {
                    memory: self.memory,
                    history: std::mem::take(&mut self.history),
                    ..Calculator::new()
                };
                return;
            }
            ClearEntry => {
                self.error = None;
                self.display = "0".to_string();
                self.fresh = true;
                return;
            }
            _ => {}
        }

        // A failed calculation locks the keypad until C or CE.
        if self.error.is_some() {
            return;
        }

        match input {
            Digit(digit) => self.type_digit(digit),
            Dot => self.type_dot(),
            Add => self.operator(Op::Add),
            Sub => self.operator(Op::Sub),
            Mul => self.operator(Op::Mul),
            Div => self.operator(Op::Div),
            Equals => self.equals(),
            Percent => self.percent(),
            Inv => self.unary(|v| {
                if v == 0.0 {
                    Err(CalcError::DivideByZero)
                } else {
                    eval::check(1.0 / v)
                }
            }),
            Sqr => self.unary(|v| eval::check(v * v)),
            Sqrt => self.unary(|v| {
                if v < 0.0 {
                    Err(CalcError::InvalidInput)
                } else {
                    Ok(v.sqrt())
                }
            }),
            Negate => {
                let v = self.value();
                if v != 0.0 {
                    self.set_value(-v);
                }
            }
            Backspace => self.backspace(),
            Clear | ClearEntry => unreachable!("handled above"),
        }
    }

    // ---------------------------------------------------------------- display

    /// What the display line shows, separators included.
    pub fn display_text(&self) -> String {
        match self.error {
            Some(error) => format!("Error: {}", error.message()),
            None => fmt::format_display(self.value()),
        }
    }

    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn value(&self) -> f64 {
        self.display.parse().unwrap_or(0.0)
    }

    // ---------------------------------------------------------------- memory

    pub fn memory_value(&self) -> f64 {
        self.memory
    }

    pub fn memory_store(&mut self) {
        self.memory = self.value();
    }

    pub fn memory_recall(&mut self) {
        if self.error.is_none() {
            self.set_value(self.memory);
        }
    }

    pub fn memory_add(&mut self) {
        self.memory += self.value();
    }

    pub fn memory_subtract(&mut self) {
        self.memory -= self.value();
    }

    pub fn memory_clear(&mut self) {
        self.memory = 0.0;
    }

    // --------------------------------------------------------------- history

    /// Newest first.
    pub fn history(&self) -> &[Entry] {
        &self.history
    }

    // --------------------------------------------------------------- private

    fn type_digit(&mut self, digit: char) {
        if self.fresh {
            self.display = digit.to_string();
            self.fresh = false;
            return;
        }
        if self.digit_count() >= MAX_INPUT_DIGITS {
            return;
        }
        if self.display == "0" {
            self.display = digit.to_string();
        } else {
            self.display.push(digit);
        }
    }

    fn type_dot(&mut self) {
        if self.fresh {
            self.display = "0.".to_string();
            self.fresh = false;
        } else if !self.display.contains('.') {
            self.display.push('.');
        }
    }

    fn operator(&mut self, op: Op) {
        let v = self.value();
        if let (Some(pending), Some(acc)) = (self.pending, self.acc) {
            // `2 + 3 +` evaluates; `2 + ×` only swaps the waiting operator.
            if !self.fresh {
                match eval::apply(acc, pending, v) {
                    Ok(result) => {
                        self.acc = Some(result);
                        self.set_value(result);
                    }
                    Err(error) => self.fail(error),
                }
            }
            self.pending = Some(op);
        } else {
            self.acc = Some(v);
            self.pending = Some(op);
            self.fresh = true;
        }
    }

    fn equals(&mut self) {
        let v = self.value();
        let (result, expression) = if let (Some(op), Some(acc)) = (self.pending, self.acc) {
            self.pending = None;
            self.acc = None;
            self.last_op = Some((op, v));
            match eval::apply(acc, op, v) {
                Ok(result) => (result, expression(acc, op, v)),
                Err(error) => return self.fail(error),
            }
        } else if let Some((op, operand)) = self.last_op {
            match eval::apply(v, op, operand) {
                Ok(result) => (result, expression(v, op, operand)),
                Err(error) => return self.fail(error),
            }
        } else {
            (v, format!("{} =", fmt::format_display(v)))
        };

        self.push_history(expression, result);
        self.set_value(result);
    }

    /// `50 + 10 %` is `50 + 50×10/100`; `50 * 10 %` scales the operand instead.
    fn percent(&mut self) {
        let v = self.value();
        let result = match (self.pending, self.acc) {
            (Some(Op::Add), Some(a)) => eval::apply(a, Op::Add, a * v / 100.0),
            (Some(Op::Sub), Some(a)) => eval::apply(a, Op::Sub, a * v / 100.0),
            (Some(Op::Mul), Some(a)) => eval::apply(a, Op::Mul, v / 100.0),
            (Some(Op::Div), Some(a)) => eval::apply(a, Op::Div, v / 100.0),
            _ => eval::check(v / 100.0),
        };

        match result {
            Ok(result) => {
                // The pending step is consumed, and a following `=` must not repeat it.
                self.pending = None;
                self.acc = None;
                self.last_op = None;
                self.set_value(result);
            }
            Err(error) => self.fail(error),
        }
    }

    fn unary(&mut self, f: impl FnOnce(f64) -> Result<f64, CalcError>) {
        match f(self.value()) {
            Ok(result) => self.set_value(result),
            Err(error) => self.fail(error),
        }
    }

    fn backspace(&mut self) {
        self.fresh = false;
        self.display.pop();
        if self.display.is_empty() || self.display == "-" {
            self.display = "0".to_string();
        }
    }

    fn digit_count(&self) -> usize {
        self.display.chars().filter(|c| c.is_ascii_digit()).count()
    }

    fn set_value(&mut self, v: f64) {
        self.display = if v == 0.0 {
            "0".to_string()
        } else {
            format!("{v}")
        };
        self.fresh = true;
    }

    fn push_history(&mut self, expression: String, result: f64) {
        self.history.insert(
            0,
            Entry {
                expression,
                result: fmt::format_display(result),
            },
        );
        self.history.truncate(HISTORY_CAP);
    }

    fn fail(&mut self, error: CalcError) {
        self.error = Some(error);
        self.pending = None;
        self.acc = None;
        self.last_op = None;
        self.fresh = true;
    }
}

fn expression(a: f64, op: Op, b: f64) -> String {
    format!(
        "{} {} {} =",
        fmt::format_display(a),
        op.symbol(),
        fmt::format_display(b)
    )
}

#[cfg(test)]
mod tests {
    use super::Input::*;
    use super::*;

    fn press(inputs: &[Input]) -> Calculator {
        let mut calc = Calculator::new();
        for input in inputs {
            calc.input(*input);
        }
        calc
    }

    fn digits(s: &str) -> Vec<Input> {
        s.chars()
            .map(|c| match c {
                '0'..='9' => Digit(c),
                '.' => Dot,
                '+' => Add,
                '-' => Sub,
                '*' => Mul,
                '/' => Div,
                '=' => Equals,
                '%' => Percent,
                _ => panic!("not an input character: {c}"),
            })
            .collect()
    }

    fn display_of(inputs: &[Input]) -> String {
        press(inputs).display_text()
    }

    #[test]
    fn typing_builds_the_display_and_groups_it() {
        assert_eq!(display_of(&digits("12345")), "12,345");
        assert_eq!(display_of(&digits("0.5")), "0.5");
    }

    #[test]
    fn a_digit_after_a_result_replaces_it() {
        assert_eq!(display_of(&digits("2+3=7")), "7");
    }

    #[test]
    fn a_third_operator_evaluates_the_pending_step() {
        assert_eq!(display_of(&digits("2+3+")), "5");
        assert_eq!(display_of(&digits("2+3+4=")), "9");
    }

    #[test]
    fn a_second_operator_only_replaces_the_first() {
        assert_eq!(display_of(&digits("2+*")), "2");
        assert_eq!(display_of(&digits("2+*3=")), "6");
    }

    #[test]
    fn equals_repeats_the_last_step() {
        assert_eq!(display_of(&digits("2+3=")), "5");
        assert_eq!(display_of(&digits("2+3==")), "8");
        assert_eq!(display_of(&digits("2+3===")), "11");
    }

    #[test]
    fn an_operator_after_equals_continues_from_the_result() {
        assert_eq!(display_of(&digits("2+3=*4=")), "20");
    }

    #[test]
    fn percent_is_context_sensitive() {
        assert_eq!(display_of(&digits("50+10%")), "55");
        assert_eq!(display_of(&digits("50-10%")), "45");
        assert_eq!(display_of(&digits("50*10%")), "5");
        assert_eq!(display_of(&digits("50/10%")), "500");
        assert_eq!(display_of(&digits("50%")), "0.5");
    }

    #[test]
    fn percent_then_equals_does_not_repeat_the_percent_step() {
        assert_eq!(display_of(&digits("50+10%=")), "55");
    }

    #[test]
    fn unary_operators_apply_immediately() {
        assert_eq!(display_of(&[Digit('2'), Inv]), "0.5");
        assert_eq!(display_of(&[Digit('9'), Sqrt]), "3");
        assert_eq!(display_of(&[Digit('7'), Sqr]), "49");
        assert_eq!(display_of(&[Digit('7'), Sqr, Digit('2')]), "2");
    }

    #[test]
    fn sqrt_of_a_negative_is_invalid_input() {
        assert_eq!(display_of(&digits("9-16=")), "-7");
        assert_eq!(
            display_of(&[Digit('9'), Sub, Digit('1'), Digit('6'), Equals, Sqrt]),
            "Error: Invalid input"
        );
    }

    #[test]
    fn divide_by_zero_locks_the_keypad() {
        let mut calc = press(&digits("1/0="));
        assert_eq!(calc.display_text(), "Error: Cannot divide by zero");
        calc.input(Digit('7'));
        assert_eq!(calc.display_text(), "Error: Cannot divide by zero");
        calc.input(ClearEntry);
        assert_eq!(calc.display_text(), "0");
    }

    #[test]
    fn inverse_of_zero_is_divide_by_zero() {
        assert_eq!(
            display_of(&[Digit('0'), Inv]),
            "Error: Cannot divide by zero"
        );
    }

    #[test]
    fn results_at_1e16_overflow() {
        assert_eq!(
            display_of(&digits("9999999999999999+1=")),
            "Error: Overflow"
        );
    }

    #[test]
    fn clear_entry_keeps_the_pending_step() {
        let mut calc = press(&digits("10+7"));
        calc.input(ClearEntry);
        assert_eq!(calc.display_text(), "0");
        calc.input(Equals);
        assert_eq!(calc.display_text(), "10");
    }

    #[test]
    fn clear_resets_but_keeps_memory_and_history() {
        let mut calc = press(&digits("2+3="));
        calc.memory_store();
        calc.input(Clear);
        assert_eq!(calc.display_text(), "0");
        assert_eq!(calc.memory_value(), 5.0);
        assert_eq!(calc.history().len(), 1);
    }

    #[test]
    fn backspace_deletes_to_zero() {
        let mut calc = press(&digits("123"));
        calc.input(Backspace);
        assert_eq!(calc.display_text(), "12");
        calc.input(Backspace);
        calc.input(Backspace);
        assert_eq!(calc.display_text(), "0");
    }

    #[test]
    fn negate_flips_the_display_not_the_accumulator() {
        assert_eq!(display_of(&digits("5-3=")), "2");
        assert_eq!(
            display_of(&[Digit('5'), Sub, Digit('3'), Negate, Equals]),
            "8"
        );
        assert_eq!(display_of(&[Digit('0'), Negate]), "0");
    }

    #[test]
    fn input_stops_at_16_digits() {
        let long = "1234567890123456789";
        assert_eq!(display_of(&digits(long)), "1,234,567,890,123,456");
    }

    #[test]
    fn history_is_newest_first_with_expression_and_result() {
        let calc = press(&digits("2+3=="));
        assert_eq!(calc.history().len(), 2);
        assert_eq!(calc.history()[0].expression, "5 + 3 =");
        assert_eq!(calc.history()[0].result, "8");
        assert_eq!(calc.history()[1].expression, "2 + 3 =");
    }

    #[test]
    fn history_caps_at_100_entries() {
        let mut calc = Calculator::new();
        for _ in 0..120 {
            calc.input(Digit('1'));
            calc.input(Equals);
        }
        assert_eq!(calc.history().len(), 100);
    }

    #[test]
    fn memory_round_trip() {
        let mut calc = press(&digits("12"));
        calc.memory_store();
        calc.input(Clear);
        calc.memory_recall();
        assert_eq!(calc.display_text(), "12");
        calc.memory_clear();
        assert_eq!(calc.memory_value(), 0.0);
    }

    #[test]
    fn no_input_sequence_panics() {
        let pool = [
            Digit('0'),
            Digit('9'),
            Dot,
            Add,
            Sub,
            Mul,
            Div,
            Equals,
            Percent,
            Inv,
            Sqr,
            Sqrt,
            Negate,
            Clear,
            ClearEntry,
            Backspace,
        ];
        // Deterministic pseudo-random walk over the input alphabet.
        let mut state = 0x2545f4914f6cdd1du64;
        for _ in 0..20_000 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let mut calc = Calculator::new();
            for _ in 0..12 {
                calc.input(pool[(state >> ((12 - 1) * 4)) as usize & (pool.len() - 1)]);
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
            }
            let text = calc.display_text();
            assert!(!text.is_empty());
        }
    }
}
