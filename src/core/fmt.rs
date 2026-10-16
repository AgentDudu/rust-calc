//! `f64` → display string: 15 significant digits, thousands grouping, `E` notation when the
//! plain form would run away.

/// Windows shows 16 significant digits; rounding to that is what makes `0.1 + 0.2` read as `0.3`
/// while still printing an exact 16-digit integer unchanged.
pub const MAX_SIG_DIGITS: usize = 16;

pub fn format_display(value: f64) -> String {
    if !value.is_finite() {
        return "Overflow".to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }

    let (digits, exp, negative) = parts(value);
    let sign = if negative { "-" } else { "" };

    // Plain decimal stays readable between 1e-9 and 1e15; outside that, switch to E notation.
    if (-9..=15).contains(&exp) {
        format!("{sign}{}", group(&plain(&digits, exp)))
    } else {
        let mantissa = match digits.split_at(1) {
            (head, "") => head.to_string(),
            (head, tail) => format!("{head}.{tail}"),
        };
        format!("{sign}{mantissa}E{exp:+}")
    }
}

/// Significant digits with trailing zeros cut, plus the base-10 exponent and the sign.
fn parts(value: f64) -> (String, i32, bool) {
    let text = format!("{:.*e}", MAX_SIG_DIGITS - 1, value);
    let (mantissa, exponent) = text
        .split_once('e')
        .expect("{:.e} always emits an exponent");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0').to_string();
    let digits = if digits.is_empty() {
        "0".into()
    } else {
        digits
    };
    (
        digits,
        exponent
            .parse()
            .expect("{:.e} always emits an integer exponent"),
        mantissa.starts_with('-'),
    )
}

/// Rebuild a plain decimal from significant digits and the exponent of the first digit.
fn plain(digits: &str, exp: i32) -> String {
    if exp >= 0 {
        let int_len = (exp as usize) + 1;
        if digits.len() > int_len {
            let (int, frac) = digits.split_at(int_len);
            format!("{int}.{frac}")
        } else {
            format!("{}{}", digits, "0".repeat(int_len - digits.len()))
        }
    } else {
        format!("0.{}{}", "0".repeat((-(exp) - 1) as usize), digits)
    }
}

/// Group the integer part in threes. Never touches the fraction or an `E`.
fn group(plain: &str) -> String {
    let (sign, rest) = match plain.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", plain),
    };
    let (int, frac) = match rest.split_once('.') {
        Some((int, frac)) => (int, Some(frac)),
        None => (rest, None),
    };

    let grouped: String = int
        .chars()
        .rev()
        .enumerate()
        .fold(String::new(), |mut acc, (i, c)| {
            if i > 0 && i % 3 == 0 {
                acc.push(',');
            }
            acc.push(c);
            acc
        })
        .chars()
        .rev()
        .collect();

    match frac {
        Some(frac) => format!("{sign}{grouped}.{frac}"),
        None => format!("{sign}{grouped}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_hides_the_f64_artifact() {
        assert_eq!(format_display(0.1 + 0.2), "0.3");
    }

    #[test]
    fn groups_the_integer_part_only() {
        assert_eq!(format_display(19_134.0), "19,134");
        assert_eq!(format_display(1_234_567.76), "1,234,567.76");
        assert_eq!(format_display(-1234.5), "-1,234.5");
        assert_eq!(format_display(0.1234), "0.1234");
    }

    #[test]
    fn zero_and_negative_zero() {
        assert_eq!(format_display(0.0), "0");
        assert_eq!(format_display(-0.0), "0");
    }

    #[test]
    fn switches_to_exponent_outside_plain_range() {
        assert_eq!(format_display(1e-10), "1E-10");
        assert_eq!(format_display(1.5e-12), "1.5E-12");
        assert_eq!(format_display(1e14), "100,000,000,000,000");
        assert_eq!(format_display(1e20), "1E+20");
    }

    #[test]
    fn keeps_16_significant_digits() {
        assert_eq!(format_display(1.0 / 3.0), "0.3333333333333333");
        assert_eq!(
            format_display(9_007_199_254_740_991.0),
            "9,007,199,254,740,991"
        );
    }

    #[test]
    fn never_panics_on_weird_input() {
        assert_eq!(format_display(f64::INFINITY), "Overflow");
        assert_eq!(format_display(f64::NAN), "Overflow");
    }
}
