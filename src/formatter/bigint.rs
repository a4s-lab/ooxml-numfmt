//! BigInt formatting for arbitrary precision integers.
//!
//! This module handles formatting of large integers that exceed f64's safe integer range (±2^53).
//! For values within the safe range, the regular f64 formatting path is used.
//! For values outside the safe range, string-based arithmetic is used to preserve precision.

use crate::ast::{FormatPart, Section};
use crate::error::FormatError;
use crate::options::FormatOptions;
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
) -> Result<String, FormatError> {
    // Check if value is within safe f64 range
    if is_safe_integer(value) {
        // Convert to f64 and use standard formatting
        let float_val: f64 = value.to_string().parse().unwrap_or(0.0);
        return super::format_number(float_val, section, opts);
    }

    // For large integers, use string-based formatting
    format_large_bigint(value, section, opts, 0)
}

/// Format a BigInt value according to a format section, with the given fill count.
pub fn format_bigint_with_fill_count(
    value: &BigInt,
    section: &Section,
    opts: &FormatOptions,
    fill_count: usize,
) -> Result<String, FormatError> {
    // Check if value is within safe f64 range
    if is_safe_integer(value) {
        // Convert to f64 and use standard formatting
        let float_val: f64 = value.to_string().parse().unwrap_or(0.0);
        return super::format_number_with_fill_count(float_val, section, opts, fill_count);
    }

    // For large integers, use string-based formatting
    format_large_bigint(value, section, opts, fill_count)
}

/// Format a BigInt value that exceeds f64's safe integer range.
/// Uses string-based arithmetic to preserve precision.
fn format_large_bigint(
    value: &BigInt,
    section: &Section,
    opts: &FormatOptions,
    fill_count: usize,
) -> Result<String, FormatError> {
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
    let formatted_integer = format_bigint_integer(
        &value_str,
        &analysis.integer_placeholders,
        analysis.has_thousands_separator,
        &analysis.inline_parts,
        opts,
        fill_count,
    );

    // Handle decimal places (for BigInt, decimal part is always 0)
    let formatted = if analysis.decimal_places() > 0 {
        let decimal = super::number::format_decimal(
            0.0,
            &analysis.decimal_placeholders,
            &analysis.decimal_inline_parts,
            opts,
            fill_count,
        );
        format!(
            "{}{}{}",
            formatted_integer, opts.locale.decimal_separator, decimal
        )
    } else {
        formatted_integer
    };

    // Build prefix
    let mut result = String::new();
    for part in &analysis.prefix_parts {
        super::number::push_part(&mut result, part, fill_count);
    }

    // Add the formatted number
    result.push_str(&formatted);

    // Build suffix
    for part in &analysis.suffix_parts {
        super::number::push_part(&mut result, part, fill_count);
    }

    Ok(result)
}

/// Format the integer part of a BigInt as a string.
fn format_bigint_integer(
    value_str: &str,
    placeholders: &[crate::ast::DigitPlaceholder],
    use_thousands: bool,
    inline_parts: &[(usize, FormatPart)],
    opts: &FormatOptions,
    fill_count: usize,
) -> String {
    let value_digits: Vec<char> = value_str.chars().collect();

    let min_digits = placeholders.iter().filter(|p| p.is_required()).count();
    let output_len = value_digits.len().max(min_digits);

    // Build right-to-left into Vec, then reverse once
    let separator_count = if use_thousands { output_len / 3 } else { 0 };
    let inline_chars: usize = inline_parts
        .iter()
        .map(|(_, part)| super::number::part_output_len(part, fill_count))
        .sum();
    let estimated_capacity = output_len + separator_count + inline_chars;
    let mut chars = Vec::with_capacity(estimated_capacity);

    // Process from right to left (least significant first)
    for (digit_count, pos_from_right) in (0..output_len).enumerate() {
        let digit_index = value_digits.len() as isize - 1 - pos_from_right as isize;

        // Add thousands separator if needed (but not at position 0)
        if use_thousands && digit_count > 0 && digit_count % 3 == 0 {
            chars.push(opts.locale.thousands_separator);
        }

        // Check if there are inline parts at this position.
        for (_, part) in inline_parts
            .iter()
            .rev()
            .filter(|(pos, _)| *pos == pos_from_right)
        {
            super::number::push_part_reversed(&mut chars, part, fill_count);
        }

        if digit_index >= 0 {
            // We have a digit from the value
            chars.push(value_digits[digit_index as usize]);
        } else {
            // Use placeholder's empty character for padding
            let placeholder_index = placeholders.len() as isize - 1 - pos_from_right as isize;
            if placeholder_index >= 0 {
                let placeholder = placeholders[placeholder_index as usize];
                if let Some(c) = placeholder.empty_char() {
                    chars.push(c);
                }
            }
        }
    }

    // Handle the case where we have no digits but need at least one
    if chars.is_empty() && placeholders.iter().any(|p| p.is_required()) {
        chars.push('0');
    }

    // Push any inline parts that are at positions beyond what we formatted.
    for (part_pos, part) in inline_parts {
        if *part_pos >= output_len {
            super::number::push_part_reversed(&mut chars, part, fill_count);
        }
    }

    // Reverse and collect into String
    chars.reverse();
    chars.into_iter().collect()
}

/// Fallback formatting for BigInt values.
/// Converts to string representation.
pub fn fallback_format_bigint(value: &BigInt) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::NumberFormat;

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

    #[test]
    fn test_format_bigint_with_fill_count() {
        let opts = FormatOptions::default();
        let large = BigInt::parse_bytes(b"123456822333333000", 10).unwrap();

        let prefix = NumberFormat::parse("*x0").unwrap();
        assert_eq!(
            prefix.format_bigint_with_fill_count(&large, &opts, 3),
            "xxx123456822333333000"
        );
        assert_eq!(
            prefix.format_bigint_with_fill_count(&-large.clone(), &opts, 3),
            "-xxx123456822333333000"
        );
        assert_eq!(
            prefix.format_bigint_with_fill_count(&BigInt::from(42), &opts, 3),
            "xxx42"
        );
        assert_eq!(prefix.format_bigint(&large, &opts), large.to_string());
        assert_eq!(
            prefix.format_bigint_with_fill_count(&large, &opts, 0),
            large.to_string()
        );

        let inline = NumberFormat::parse("0*x0").unwrap();
        assert_eq!(
            inline.format_bigint_with_fill_count(&large, &opts, 3),
            "12345682233333300xxx0"
        );

        let after_decimal = NumberFormat::parse("0.*x00").unwrap();
        assert_eq!(
            after_decimal.format_bigint_with_fill_count(&large, &opts, 3),
            "123456822333333000.xxx00"
        );

        let suffix = NumberFormat::parse("0*x").unwrap();
        assert_eq!(
            suffix.format_bigint_with_fill_count(&large, &opts, 3),
            "123456822333333000xxx"
        );

        let unicode = NumberFormat::parse("*한0").unwrap();
        assert_eq!(
            unicode.format_bigint_with_fill_count(&large, &opts, 3),
            "한한한123456822333333000"
        );
    }
}
