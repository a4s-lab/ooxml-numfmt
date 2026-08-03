//! Format value formatting engine

mod date;
mod fraction;
mod number;
mod text;

#[cfg(feature = "bigint")]
mod bigint;

pub use number::format_number;

#[cfg(feature = "bigint")]
#[allow(unused_imports)]
pub use bigint::{fallback_format_bigint, format_bigint, is_safe_integer};

use crate::ast::{FormatPart, NumberFormat, Section};
use crate::error::FormatError;
use crate::options::FormatOptions;

impl NumberFormat {
    /// Format a numeric value using this format code.
    ///
    /// This is an infallible method that returns a formatted string.
    /// For date formats or when precise error handling is needed,
    /// use `try_format()` instead.
    pub fn format(&self, value: f64, opts: &FormatOptions) -> String {
        match self.try_format(value, opts) {
            Ok(result) => result,
            Err(_) => fallback_format(value),
        }
    }

    /// Try to format a numeric value using this format code.
    ///
    /// Returns an error if the format cannot be applied to the value.
    pub fn try_format(&self, value: f64, opts: &FormatOptions) -> Result<String, FormatError> {
        // Handle special float values
        if value.is_nan() {
            return Ok("NaN".to_string());
        }
        if value.is_infinite() {
            return Ok(if value.is_sign_positive() {
                "Infinity"
            } else {
                "-Infinity"
            }
            .to_string());
        }

        // Select the appropriate section based on value
        let sections = self.sections();
        let section = &sections[self.select_section_index(value)];

        // Excel behavior: when a conditional section strictly matches, format using absolute value
        // Use absolute value only when the condition is strictly satisfied (not at boundary)
        let has_conditions = sections.iter().any(|s| s.condition().is_some());
        let use_abs_value = has_conditions
            && section.condition().is_some()
            && section.condition().unwrap().is_strict_match(value);
        let format_value = if use_abs_value { value.abs() } else { value };

        // Handle "General" format (empty section with no parts)
        // This uses fallback formatting which matches Excel's General behavior
        // Note: sections can have conditions or colors and still be General format
        if section.parts().is_empty() {
            // Special case: if this is a strict conditional match, Excel truncates decimals
            // This handles formats like "[<-25]General" which show "50" instead of "50.1"
            let truncated_value = if use_abs_value && format_value.fract() != 0.0 {
                format_value.trunc()
            } else {
                format_value
            };
            return Ok(fallback_format(truncated_value));
        }

        // Check if this is a date format
        if section.has_date_parts() {
            return date::format_date(format_value, section, opts);
        }

        // Determine if we need to add a minus sign
        // For single-section formats, we add the minus sign ourselves
        // For multi-section formats, the section handles it
        // For literal-only formats (no numeric parts), add minus ONLY if it's a single unescaped single-char literal
        // But NOT if we're using absolute value due to conditional matching
        // EXCEPTION: Fraction and scientific notation formats add their own minus sign
        let num_sections = sections.len();
        let has_numeric_parts = section.parts().iter().any(|p| p.is_numeric_part());
        let is_single_char_literal = section.parts().len() == 1
            && matches!(&section.parts()[0], FormatPart::Literal(s) if s.len() == 1);
        let has_fraction = section
            .parts()
            .iter()
            .any(|p| matches!(p, FormatPart::Fraction { .. }));
        let has_scientific = section
            .parts()
            .iter()
            .any(|p| matches!(p, FormatPart::Scientific { .. }));
        let need_minus_sign = num_sections == 1
            && value < 0.0
            && (has_numeric_parts || is_single_char_literal)
            && !use_abs_value
            && !has_fraction
            && !has_scientific;

        // Format as a number
        let mut result = format_number(format_value, section, opts)?;

        // Add minus sign for single-section formats with negative values
        // Note: format_number uses abs(value), so it never includes the minus sign
        // Exception: Fraction and scientific notation formats add their own minus sign
        if need_minus_sign {
            result.insert(0, '-');
        }

        Ok(result)
    }

    /// Return the index of the appropriate format section based on the value.
    ///
    /// Section selection rules:
    /// - 1 section: used for all values
    /// - 2 sections: first for positive/zero, second for negative
    /// - 3 sections: positive, negative, zero
    /// - 4 sections: positive, negative, zero, text
    pub fn select_section_index(&self, value: f64) -> usize {
        let sections = self.sections();

        // Check if any section has conditions
        let has_conditions = sections.iter().any(|s| s.condition().is_some());

        if has_conditions {
            // With conditions: find matching conditional, or first non-conditional
            for (index, section) in sections.iter().enumerate() {
                if let Some(condition) = section.condition() {
                    if condition.evaluate(value) {
                        return index;
                    }
                } else {
                    // No condition on this section - use it as fallback
                    return index;
                }
            }
            // Fallback to last section if nothing matched
            return sections.len() - 1;
        }

        // Standard section selection based on value sign (no conditions)
        match sections.len() {
            0 => unreachable!("NumberFormat should always have at least one section"),
            1 => 0,
            2 => {
                if value < 0.0 {
                    1
                } else {
                    0
                }
            }
            3 | 4 => {
                if value > 0.0 {
                    0
                } else if value < 0.0 {
                    1
                } else {
                    // Zero value - use section[2]
                    // Unless it's text-only (@), then use positive section
                    if sections[2].has_text_placeholder()
                        && !sections[2].parts().iter().any(|p| {
                            p.is_numeric_part()
                                || matches!(
                                    p,
                                    FormatPart::Literal(_) | FormatPart::EscapedLiteral(_)
                                )
                        })
                    {
                        0
                    } else {
                        2
                    }
                }
            }
            _ => 0,
        }
    }

    /// Format a text value using this format code.
    pub fn format_text(&self, text: &str, _opts: &FormatOptions) -> String {
        if let Some(text_section) = self.select_text_section() {
            let mut result = String::new();

            for part in text_section.parts() {
                match part {
                    FormatPart::TextPlaceholder => result.push_str(text),
                    FormatPart::Literal(s) | FormatPart::EscapedLiteral(s) => result.push_str(s),
                    _ => {}
                }
            }

            result
        } else {
            // Default: return text as-is
            text.to_string()
        }
    }

    /// Selects the text section with the following policy:
    /// - If the 4th section is present, always return it.
    /// - With fewer sections, use the final section only if it contains `@`.
    /// - Otherwise, return None.
    fn select_text_section(&self) -> Option<&Section> {
        let sections = self.sections();

        // Text section is the 4th section if present
        if sections.len() >= 4 {
            return Some(&sections[3]);
        }

        // With fewer sections, the final section is the text section only if it contains `@`.
        sections
            .last()
            .filter(|section| section.has_text_placeholder())
    }

    /// Format a BigInt value using this format code (requires `bigint` feature).
    ///
    /// For values within f64's safe integer range (±2^53), converts to f64 and uses
    /// standard formatting. For larger values, uses string-based formatting to
    /// preserve precision.
    #[cfg(feature = "bigint")]
    pub fn format_bigint(&self, value: &num_bigint::BigInt, opts: &FormatOptions) -> String {
        match self.try_format_bigint(value, opts) {
            Ok(result) => result,
            Err(_) => bigint::fallback_format_bigint(value),
        }
    }

    /// Try to format a BigInt value using this format code (requires `bigint` feature).
    ///
    /// For values within f64's safe integer range (±2^53), converts to f64 and uses
    /// standard formatting. For larger values, uses string-based formatting to
    /// preserve precision.
    #[cfg(feature = "bigint")]
    pub fn try_format_bigint(
        &self,
        value: &num_bigint::BigInt,
        opts: &FormatOptions,
    ) -> Result<String, FormatError> {
        use num_bigint::Sign;

        // Check if value is within safe f64 range
        if bigint::is_safe_integer(value) {
            // Convert to f64 and use standard formatting
            let float_val: f64 = value.to_string().parse().unwrap_or(0.0);
            return self.try_format(float_val, opts);
        }

        // For large integers, use string-based formatting
        let is_negative = value.sign() == Sign::Minus;
        let section = if is_negative {
            // Select negative section if available
            let sections = self.sections();
            if sections.len() >= 2 {
                &sections[1]
            } else {
                &sections[0]
            }
        } else {
            &self.sections()[0]
        };

        // Handle "General" format (empty section with no parts)
        if section.parts().is_empty() {
            return Ok(bigint::fallback_format_bigint(value));
        }

        // Check if this is a date format - BigInt can't be used for dates
        if section.has_date_parts() {
            return Err(FormatError::TypeMismatch {
                expected: "numeric format",
                got: "date format with BigInt value",
            });
        }

        // Format using BigInt-specific logic
        let mut result = bigint::format_bigint(value, section, opts)?;

        // Add minus sign for negative values in single-section formats
        let sections = self.sections();
        let has_numeric_parts = section.parts().iter().any(|p| p.is_numeric_part());
        if sections.len() == 1 && is_negative && has_numeric_parts {
            result.insert(0, '-');
        }

        Ok(result)
    }
}

/// Fallback formatting for when the format code cannot be applied.
///
/// Implements Excel's "General" number format behavior:
/// - Very small numbers (0 < |x| < 1E-4) use scientific notation
/// - Exact integers below 1E11 are displayed without scientific notation
/// - Values at or above 1E11 use scientific notation
/// - No trailing zeros after decimal point
pub fn fallback_format(value: f64) -> String {
    // Handle zero and non-finite values before decimal precision calculations.
    if value == 0.0 {
        return "0".to_string();
    }
    if !value.is_finite() {
        return value.to_string();
    }

    // Integer fast path for exact values below the scientific boundary.
    let int_val = value.trunc() as i64;
    if (value - int_val as f64).abs() < f64::EPSILON && value.abs() >= 1.0 && value.abs() < 1e11 {
        return int_val.to_string();
    }

    let abs_value = value.abs();

    // Choose fixed precision and round before deciding which notation to return.
    // Excel's General format shows up to 11 characters total (including decimal point)
    // but we need to be smart about significant figures.
    let decimal_formatted = if abs_value >= 1.0 {
        // For numbers >= 1, format with appropriate decimal places.
        let integer_digits = abs_value.log10().floor() as usize + 1;
        let decimal_places = if integer_digits >= 10 {
            0
        } else {
            (10 - integer_digits).min(10)
        };
        format!("{:.prec$}", value, prec = decimal_places)
    } else {
        // For numbers < 1, format with up to 9 decimal places (to fit in 11 chars: "0." + 9 digits)
        // Excel's limit is 11 chars for the numeric part, not counting the sign
        // So negative numbers can be up to 12 chars total
        let max_decimals = 9;
        let test_format = format!("{:.prec$}", value, prec = max_decimals);

        // Check length of numeric part only (excluding sign for negative numbers)
        let numeric_part = if value < 0.0 {
            &test_format[1..] // Skip the '-' sign
        } else {
            &test_format[..]
        };

        // If numeric part exceeds 11 chars, reduce decimal places
        if numeric_part.len() > 11 {
            let excess = numeric_part.len() - 11;
            let reduced_decimals = max_decimals.saturating_sub(excess);
            format!("{:.prec$}", value, prec = reduced_decimals)
        } else {
            test_format
        }
    };

    let rounded_reaches_boundary = decimal_formatted
        .parse::<f64>()
        .map(|rounded| rounded.abs() >= 1e11)
        .unwrap_or(false);

    // Very small values use scientific notation when their decimal representation
    // does not fit within General's 11-character limit.
    let small_value_needs_scientific = if abs_value > 0.0 && abs_value < 0.0001 {
        let test_str = format!("{:.15}", abs_value);
        // Trim trailing zeros
        let trimmed = test_str.trim_end_matches('0').trim_end_matches('.');

        // If it doesn't fit in 11 chars, use scientific notation
        trimmed.len() > 11
    } else {
        false
    };

    let use_scientific = rounded_reaches_boundary || small_value_needs_scientific;

    if use_scientific {
        // Format in scientific notation with up to 5 decimal places
        // Excel shows "1.23457E+12" format
        let formatted = format!("{:.5E}", value);

        // Excel uses specific scientific notation format:
        // Remove trailing zeros from mantissa, but keep at least one decimal place
        if let Some(e_pos) = formatted.find('E') {
            let (mantissa, exponent) = formatted.split_at(e_pos);
            let trimmed_mantissa = mantissa.trim_end_matches('0');
            let final_mantissa = trimmed_mantissa
                .strip_suffix('.')
                .unwrap_or(trimmed_mantissa);

            // Format exponent to match Excel: E+12, E-05, etc.
            let exp_str = &exponent[1..]; // Skip 'E'
            let exp_value: i32 = exp_str.parse().unwrap_or(0);
            format!("{}E{:+03}", final_mantissa, exp_value)
        } else {
            formatted
        }
    } else {
        // Use decimal notation

        // Trim trailing zeros after decimal point
        if decimal_formatted.contains('.') {
            let trimmed = decimal_formatted.trim_end_matches('0');
            if trimmed.ends_with('.') {
                trimmed.trim_end_matches('.').to_string()
            } else {
                trimmed.to_string()
            }
        } else {
            decimal_formatted
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Condition, DigitPlaceholder, Section};

    fn make_format(sections: Vec<Section>) -> NumberFormat {
        NumberFormat::from_sections(sections)
    }

    fn make_section(parts: Vec<FormatPart>) -> Section {
        Section::new(None, None, parts)
    }

    #[test]
    fn test_select_section_single() {
        let fmt = make_format(vec![make_section(vec![FormatPart::Digit(
            DigitPlaceholder::Zero,
        )])]);

        let opts = FormatOptions::default();
        // Single-section formats: negative values get a minus sign prefix
        assert_eq!(fmt.format(42.0, &opts), "42");
        assert_eq!(fmt.format(-42.0, &opts), "-42");
        assert_eq!(fmt.format(0.0, &opts), "0");
    }

    #[test]
    fn test_select_section_two_sections() {
        let fmt = make_format(vec![
            make_section(vec![FormatPart::Digit(DigitPlaceholder::Zero)]),
            make_section(vec![
                FormatPart::Literal("-".to_string()),
                FormatPart::Digit(DigitPlaceholder::Zero),
            ]),
        ]);

        let opts = FormatOptions::default();
        assert_eq!(fmt.format(42.0, &opts), "42");
        assert_eq!(fmt.format(-42.0, &opts), "-42");
        assert_eq!(fmt.format(0.0, &opts), "0");
    }

    #[test]
    fn test_select_section_three_sections() {
        let fmt = make_format(vec![
            make_section(vec![
                FormatPart::Literal("+".to_string()),
                FormatPart::Digit(DigitPlaceholder::Zero),
            ]),
            make_section(vec![
                FormatPart::Literal("-".to_string()),
                FormatPart::Digit(DigitPlaceholder::Zero),
            ]),
            make_section(vec![FormatPart::Literal("ZERO".to_string())]),
        ]);

        let opts = FormatOptions::default();
        assert_eq!(fmt.format(42.0, &opts), "+42");
        assert_eq!(fmt.format(-42.0, &opts), "-42");
        assert_eq!(fmt.format(0.0, &opts), "ZERO");
    }

    #[test]
    fn test_select_section_with_condition() {
        let fmt = make_format(vec![
            Section::new(
                Some(Condition::GreaterThan(100.0)),
                None,
                vec![FormatPart::Literal("BIG".to_string())],
            ),
            make_section(vec![FormatPart::Digit(DigitPlaceholder::Zero)]),
        ]);

        let opts = FormatOptions::default();
        assert_eq!(fmt.format(150.0, &opts), "BIG");
        assert_eq!(fmt.format(50.0, &opts), "50");
    }

    #[test]
    fn test_fallback_format() {
        assert_eq!(fallback_format(42.0), "42");
        assert_eq!(fallback_format(42.5), "42.5");
        assert_eq!(fallback_format(42.123456), "42.123456");
    }

    #[test]
    fn test_format_text() {
        let fmt = make_format(vec![
            make_section(vec![FormatPart::Digit(DigitPlaceholder::Zero)]),
            make_section(vec![FormatPart::Digit(DigitPlaceholder::Zero)]),
            make_section(vec![FormatPart::Digit(DigitPlaceholder::Zero)]),
            make_section(vec![
                FormatPart::Literal("<<".to_string()),
                FormatPart::TextPlaceholder,
                FormatPart::Literal(">>".to_string()),
            ]),
        ]);

        let opts = FormatOptions::default();
        assert_eq!(fmt.format_text("hello", &opts), "<<hello>>");
    }

    #[test]
    fn test_format_text_with_placeholder_in_single_section() {
        let fmt = NumberFormat::parse("\"pre\"@\"post\"").unwrap();
        let opts = FormatOptions::default();

        assert_eq!(fmt.format_text("hello", &opts), "prehellopost");
    }

    #[test]
    fn test_format_text_with_placeholder_in_second_section() {
        let fmt = NumberFormat::parse("0;\"pre\"@").unwrap();
        let opts = FormatOptions::default();

        assert_eq!(fmt.format_text("hello", &opts), "prehello");
    }

    #[test]
    fn test_format_text_with_placeholder_in_third_section() {
        let fmt = NumberFormat::parse("0;0;\"pre\"@").unwrap();
        let opts = FormatOptions::default();

        assert_eq!(fmt.format_text("hello", &opts), "prehello");
    }

    #[test]
    fn test_format_text_with_literal_only_fourth_section() {
        let fmt = NumberFormat::parse("0;0;0;\"literal\"").unwrap();
        let opts = FormatOptions::default();

        assert_eq!(fmt.format_text("hello", &opts), "literal");
    }

    #[test]
    fn test_format_text_with_explicitly_empty_fourth_section() {
        let fmt = NumberFormat::parse("0;0;0;").unwrap();
        let opts = FormatOptions::default();

        assert_eq!(fmt.format_text("hello", &opts), "");
    }
}
