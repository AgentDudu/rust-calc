//! Binary operators and the magnitude guard every result passes through.

use crate::core::error::CalcError;

/// Windows Calculator refuses results at or beyond 1e16.
pub const MAX_MAGNITUDE: f64 = 1e16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

impl Op {
    #[must_use]
    pub fn symbol(self) -> char {
        match self {
            Op::Add => '+',
            Op::Sub => '-',
            Op::Mul => '×',
            Op::Div => '÷',
        }
    }
}

/// `a op b`, with the magnitude guard applied to whatever comes out.
///
/// # Errors
///
/// [`CalcError::DivideByZero`] when `op` is division and `b` is zero, plus whatever [`check`]
/// rejects about the result.
pub fn apply(a: f64, op: Op, b: f64) -> Result<f64, CalcError> {
    let r = match op {
        Op::Add => a + b,
        Op::Sub => a - b,
        Op::Mul => a * b,
        Op::Div => {
            if b == 0.0 {
                return Err(CalcError::DivideByZero);
            }
            a / b
        }
    };
    check(r)
}

/// Single place where a number stops being displayable.
///
/// # Errors
///
/// [`CalcError::InvalidInput`] for NaN; [`CalcError::Overflow`] for an infinite value or one at or
/// beyond [`MAX_MAGNITUDE`].
pub fn check(v: f64) -> Result<f64, CalcError> {
    if v.is_nan() {
        Err(CalcError::InvalidInput)
    } else if v.is_infinite() || v.abs() >= MAX_MAGNITUDE {
        Err(CalcError::Overflow)
    } else {
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divide_by_zero_is_an_error() {
        assert_eq!(apply(1.0, Op::Div, 0.0), Err(CalcError::DivideByZero));
        assert_eq!(apply(1.0, Op::Div, -0.0), Err(CalcError::DivideByZero));
    }

    #[test]
    fn results_at_or_above_1e16_overflow() {
        assert_eq!(check(1e16), Err(CalcError::Overflow));
        assert_eq!(check(-1e16), Err(CalcError::Overflow));
        // 9999999999999999 is not representable: the nearest f64 is exactly 1e16.
        assert_eq!(9_999_999_999_999_999.0, 1e16);
        assert_eq!(check(9_007_199_254_740_991.0), Ok(9_007_199_254_740_991.0));
        assert_eq!(
            apply(9_999_999_999_999_999.0, Op::Add, 1.0),
            Err(CalcError::Overflow)
        );
    }

    #[test]
    fn arithmetic() {
        assert_eq!(apply(2.0, Op::Add, 3.0), Ok(5.0));
        assert_eq!(apply(2.0, Op::Sub, 3.0), Ok(-1.0));
        assert_eq!(apply(2.0, Op::Mul, 3.0), Ok(6.0));
        assert_eq!(apply(1.0, Op::Div, 4.0), Ok(0.25));
    }
}
