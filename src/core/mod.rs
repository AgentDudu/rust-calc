//! Pure calculator engine. No `egui` here — the UI layer only renders this state.

pub mod calc;
pub mod error;
pub mod eval;
pub mod fmt;

pub use calc::{Calculator, Entry, Input};
pub use error::CalcError;
