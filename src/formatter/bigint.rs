//! BigInt formatting for arbitrary precision integers.
//!
//! This module handles formatting of large integers that exceed f64's safe integer range (±2^53).
//! For values within the safe range, the regular f64 formatting path is used.
//! For values outside the safe range, string-based arithmetic is used to preserve precision.

use crate::ast::Section;
use crate::error::FormatError;
use crate::options::FormatOptions;
use crate::output::{output_for_part, push_output, FormatOutput};
use num_bigint::BigInt;

/// The maximum safe integer value for f64 (2^53 - 1)
pub const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
/// The minimum safe integer value for f64 (-(2^53 - 1))
pub const MIN_SAFE_INTEGER: i64 = -9_007_199_254_740_991;

/// Check if a BigInt is within the safe f64 integer range.
pub fn is_safe_integer(n: &BigInt) -> bool {
    let min_safe = BigInt::from(MIN_SAFE_INTEGER);
    let max_safe = BigInt::from(MAX_SAFE_INTEGER);
    n >= &min_safe && n <= &max_safe
}

/// Format a BigInt value according to a format section.
///
/// For values within safe f64 range, converts to f64 and uses standard formatting.
/// For values outside safe range, uses string-based formatting to preserve precision.
pub fn format_bigint(
    value: &BigInt,
    section: &Section,
    opts: &FormatOptions,
) -> Result<Vec<FormatOutput>, FormatError> {
    // Check if value is within safe f64 range
    if is_safe_integer(value) {
        // Convert to f64 and use standard formatting
        let float_val: f64 = value.to_string().parse().unwrap_or(0.0);
        return super::format_number(float_val, section, opts);
    }

    // For large integers, use string-based formatting
    format_large_bigint(value, section, opts)
}

/// Format a BigInt value that exceeds f64's safe integer range.
/// Uses string-based arithmetic to preserve precision.
fn format_large_bigint(
    value: &BigInt,
    section: &Section,
    opts: &FormatOptions,
) -> Result<Vec<FormatOutput>, FormatError> {
    use num_bigint::Sign;

    let is_negative = value.sign() == Sign::Minus;
    let abs_value = if is_negative {
        -value.clone()
    } else {
        value.clone()
    };

    // Analyze the format to understand what we need to do
    let analysis = super::number::analyze_format(section);

    // Apply thousands scaling (trailing commas divide by 1000 each)
    let scaled_value = if analysis.thousands_scale > 0 {
        let divisor = BigInt::from(1000_u64).pow(analysis.thousands_scale as u32);
        &abs_value / &divisor
    } else {
        abs_value.clone()
    };

    // Convert to string for formatting
    let value_str = scaled_value.to_string();

    // Format the integer part
    let mut formatted = format_bigint_integer(
        &value_str,
        &analysis.integer_placeholders,
        analysis.has_thousands_separator,
        &analysis.integer_inline_parts,
        opts,
    );

    // Handle decimal places (for BigInt, decimal part is always 0)
    if analysis.decimal_places() > 0 {
        push_output(
            &mut formatted,
            FormatOutput::Text(opts.locale.decimal_separator.to_string()),
        );
        for output in super::number::format_decimal(
            0.0,
            &analysis.decimal_placeholders,
            &analysis.decimal_inline_parts,
            opts,
        ) {
            push_output(&mut formatted, output);
        }
    }

    let capacity = analysis.prefix_parts.len() + formatted.len() + analysis.suffix_parts.len();
    let mut result = Vec::with_capacity(capacity);

    result.extend(analysis.prefix_parts.iter().filter_map(output_for_part));
    result.extend(formatted);
    result.extend(analysis.suffix_parts.iter().filter_map(output_for_part));

    Ok(result)
}

/// Format the integer part of a BigInt.
fn format_bigint_integer(
    value_str: &str,
    placeholders: &[crate::ast::DigitPlaceholder],
    use_thousands: bool,
    inline_parts: &[(usize, crate::ast::FormatPart)],
    opts: &FormatOptions,
) -> Vec<FormatOutput> {
    let value_digits: Vec<char> = value_str.chars().collect();
    let min_digits = placeholders.iter().filter(|p| p.is_required()).count();
    let output_len = value_digits.len().max(min_digits);

    super::number::format_integer_digits(
        &value_digits,
        placeholders,
        use_thousands,
        inline_parts,
        output_len,
        opts,
    )
}

/// Fallback formatting for BigInt values.
/// Converts to string representation.
pub fn fallback_format_bigint(value: &BigInt) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_safe_integer() {
        assert!(is_safe_integer(&BigInt::from(0)));
        assert!(is_safe_integer(&BigInt::from(1000)));
        assert!(is_safe_integer(&BigInt::from(-1000)));
        assert!(is_safe_integer(&BigInt::from(MAX_SAFE_INTEGER)));
        assert!(is_safe_integer(&BigInt::from(MIN_SAFE_INTEGER)));

        // Just outside safe range
        let above_max = BigInt::from(MAX_SAFE_INTEGER) + 1;
        let below_min = BigInt::from(MIN_SAFE_INTEGER) - 1;
        assert!(!is_safe_integer(&above_max));
        assert!(!is_safe_integer(&below_min));

        // Large values
        assert!(!is_safe_integer(
            &BigInt::parse_bytes(b"123456822333333000", 10).unwrap()
        ));
    }

    #[test]
    fn test_fallback_format_bigint() {
        let big = BigInt::parse_bytes(b"123456822333333000", 10).unwrap();
        assert_eq!(fallback_format_bigint(&big), "123456822333333000");
    }
}
