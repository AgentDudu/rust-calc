//! User-facing failure states. `std::` paths only — a `core` module shadows `core::` in this crate.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcError {
    DivideByZero,
    InvalidInput,
    Overflow,
}

impl CalcError {
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            CalcError::DivideByZero => "Cannot divide by zero",
            CalcError::InvalidInput => "Invalid input",
            CalcError::Overflow => "Overflow",
        }
    }
}

impl std::fmt::Display for CalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}
