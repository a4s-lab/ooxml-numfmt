//! Number formatting (integers, decimals, percentages, scientific notation)

use crate::ast::{DigitPlaceholder, FormatPart, Section};
use crate::error::FormatError;
use crate::options::FormatOptions;

/// Format a simple integer value with digit placeholders (no separators or literals).
/// Based on SSF's write_num helper in bits/59_numhelp.js.
/// Maps digits to placeholders from right to left, using placeholder padding for missing digits.
pub(crate) fn format_simple_with_placeholders(
    value: u64,
    placeholders: &[DigitPlaceholder],
) -> String {
    if placeholders.is_empty() {
        return value.to_string();
    }

    let value_str = value.to_string();
    let value_digits: Vec<char> = value_str.chars().collect();

    // If we have more digits than placeholders, show all digits
    if value_digits.len() > placeholders.len() {
        return value_str;
    }

    // Build right-to-left into Vec, then reverse once (O(n) instead of O(n²) with insert(0))
    let mut chars = Vec::with_capacity(placeholders.len());

    // Process from right to left
    for pos_from_right in 0..placeholders.len() {
        let digit_index = value_digits.len() as isize - 1 - pos_from_right as isize;
        let placeholder_index = placeholders.len() - 1 - pos_from_right;
        let placeholder = placeholders[placeholder_index];

        if digit_index >= 0 {
            // We have a digit from the value
            chars.push(value_digits[digit_index as usize]);
        } else {
            // Use placeholder's empty character for padding
            if let Some(c) = placeholder.empty_char() {
                chars.push(c);
            }
        }
    }

    chars.reverse();
    chars.into_iter().collect()
}

/// Analysis of a format section's numeric structure.
#[derive(Debug, Clone)]
pub struct FormatAnalysis {
    /// Number of integer digit placeholders
    pub integer_placeholders: Vec<DigitPlaceholder>,
    /// Number of decimal digit placeholders
    pub decimal_placeholders: Vec<DigitPlaceholder>,
    /// Whether the format has a thousands separator
    pub has_thousands_separator: bool,
    /// Number of percent signs (each multiplies by 100)
    pub percent_count: usize,
    /// Thousands scaling factor (trailing commas divide by 1000 each)
    pub thousands_scale: usize,
    /// Parts that appear inline with integer digits (position -> part)
    /// Position is counted from the right (0 = ones place, 1 = tens, etc.)
    pub inline_parts: Vec<(usize, FormatPart)>,
    /// Parts that appear inline with decimal digits (position -> part)
    /// Position is counted from the left (0 = first decimal place, 1 = second, etc.)
    pub decimal_inline_parts: Vec<(usize, FormatPart)>,
    /// Parts before the number (literals, etc.)
    pub prefix_parts: Vec<FormatPart>,
    /// Parts after the number (literals, percent, etc.)
    pub suffix_parts: Vec<FormatPart>,
}

impl FormatAnalysis {
    /// Get the number of required decimal places
    pub fn decimal_places(&self) -> usize {
        self.decimal_placeholders.len()
    }

    /// Get the minimum integer digits (count of Zero placeholders)
    #[allow(dead_code)]
    pub fn min_integer_digits(&self) -> usize {
        self.integer_placeholders
            .iter()
            .filter(|p| p.is_required())
            .count()
    }
}

pub(super) fn inline_part_text(part: &FormatPart) -> Option<&str> {
    match part {
        FormatPart::Literal(text) | FormatPart::EscapedLiteral(text) => Some(text),
        FormatPart::Locale(locale) => locale.currency.as_deref(),
        _ => None,
    }
}

pub(super) fn push_part(output: &mut String, part: &FormatPart, fill_count: usize) {
    if let Some(text) = inline_part_text(part) {
        output.push_str(text);
        return;
    }

    match part {
        FormatPart::Percent => output.push('%'),
        FormatPart::Skip(_) => output.push(' '),
        FormatPart::Fill(character) => {
            output.extend(std::iter::repeat_n(*character, fill_count));
        }
        _ => {}
    }
}

pub(super) fn push_part_reversed(output: &mut Vec<char>, part: &FormatPart, fill_count: usize) {
    if let Some(text) = inline_part_text(part) {
        output.extend(text.chars().rev());
        return;
    }

    match part {
        FormatPart::Percent => output.push('%'),
        FormatPart::Skip(_) => output.push(' '),
        FormatPart::Fill(character) => {
            output.extend(std::iter::repeat_n(*character, fill_count));
        }
        _ => {}
    }
}

pub(super) fn part_output_len(part: &FormatPart, fill_count: usize) -> usize {
    if let Some(text) = inline_part_text(part) {
        return text.len();
    }

    match part {
        FormatPart::Percent | FormatPart::Skip(_) => 1,
        FormatPart::Fill(character) => character.len_utf8() * fill_count,
        _ => 0,
    }
}

/// Analyze a format section to extract its numeric structure.
pub fn analyze_format(section: &Section) -> FormatAnalysis {
    let mut integer_placeholders = Vec::new();
    let mut decimal_placeholders = Vec::new();
    let mut has_thousands_separator = false;
    let mut percent_count = 0;
    let mut inline_parts = Vec::new();
    let mut decimal_inline_parts = Vec::new();
    let mut prefix_parts = Vec::new();
    let mut suffix_parts = Vec::new();
    let active_fill = section.active_fill();
    let last_numeric = section
        .parts
        .iter()
        .rposition(|part| matches!(part, FormatPart::Digit(_) | FormatPart::DecimalPoint));

    // First, count trailing commas by scanning backwards from the end
    // Any ThousandsSeparator after the last Digit/DecimalPoint is a trailing comma
    let mut trailing_comma_count = 0;
    for part in section.parts.iter().rev() {
        match part {
            FormatPart::ThousandsSeparator => {
                trailing_comma_count += 1;
            }
            FormatPart::Digit(_) | FormatPart::DecimalPoint => {
                // Found a digit or decimal, stop counting trailing commas
                break;
            }
            _ => {
                // Other parts (Fill, Skip, Literal) - continue scanning
            }
        }
    }

    // Track which commas are trailing (to exclude from has_thousands_separator)
    let mut commas_seen = 0;
    let total_commas = section
        .parts
        .iter()
        .filter(|p| matches!(p, FormatPart::ThousandsSeparator))
        .count();
    let non_trailing_comma_count = total_commas - trailing_comma_count;

    let mut seen_digit = false;
    let mut after_decimal = false;
    let mut after_digits = false;

    for (index, part) in section.parts.iter().enumerate() {
        match part {
            FormatPart::Digit(placeholder) => {
                seen_digit = true;
                after_digits = false;
                if after_decimal {
                    decimal_placeholders.push(*placeholder);
                } else {
                    integer_placeholders.push(*placeholder);
                }
            }
            FormatPart::DecimalPoint => {
                after_decimal = true;
                seen_digit = true;
                after_digits = true; // Mark that integer digit sequence is complete
            }
            FormatPart::ThousandsSeparator => {
                commas_seen += 1;
                // Only count as thousands separator if it's not a trailing comma
                // Trailing commas are only for scaling, not for formatting separators
                if commas_seen <= non_trailing_comma_count {
                    has_thousands_separator = true;
                }
            }
            FormatPart::Percent => {
                percent_count += 1;
                if seen_digit {
                    after_digits = true;
                    suffix_parts.push(part.clone());
                } else {
                    prefix_parts.push(part.clone());
                }
            }
            FormatPart::Literal(_)
            | FormatPart::EscapedLiteral(_)
            | FormatPart::Locale(crate::ast::LocaleCode {
                currency: Some(_), ..
            }) => {
                if !seen_digit {
                    // Before any digits - prefix
                    prefix_parts.push(part.clone());
                } else if after_digits {
                    // After all digits (after decimal or after digit sequence ended) - suffix
                    suffix_parts.push(part.clone());
                } else if after_decimal {
                    // Among decimal digits - inline part in decimal part
                    // Store position from left (index in decimal_placeholders)
                    decimal_inline_parts.push((decimal_placeholders.len(), part.clone()));
                } else {
                    // Among integer digits - inline part
                    // Store the current placeholder count - we'll convert to position later
                    inline_parts.push((integer_placeholders.len(), part.clone()));
                }
            }
            FormatPart::Locale(loc) if loc.currency.is_none() => {
                // Locale without currency - treat as before
                if !seen_digit {
                    prefix_parts.push(part.clone());
                } else if after_digits {
                    suffix_parts.push(part.clone());
                }
            }
            FormatPart::Skip(c) => {
                // Skip adds space equivalent to character width
                if !seen_digit {
                    prefix_parts.push(FormatPart::Literal(" ".to_string()));
                } else {
                    suffix_parts.push(FormatPart::Literal(" ".to_string()));
                }
                let _ = c; // suppress unused warning
            }
            FormatPart::Fill(_) if active_fill == Some(index) => {
                if !seen_digit {
                    prefix_parts.push(part.clone());
                } else if last_numeric.is_some_and(|last| index > last) {
                    suffix_parts.push(part.clone());
                } else if after_decimal {
                    decimal_inline_parts.push((decimal_placeholders.len(), part.clone()));
                } else {
                    inline_parts.push((integer_placeholders.len(), part.clone()));
                }
            }
            FormatPart::Fill(_) => {}
            _ => {
                // Handle other parts as literals in prefix/suffix
                if !seen_digit {
                    prefix_parts.push(part.clone());
                } else if after_digits {
                    suffix_parts.push(part.clone());
                }
            }
        }
    }

    // Ensure we have at least one integer placeholder for output
    if integer_placeholders.is_empty() && !after_decimal {
        integer_placeholders.push(DigitPlaceholder::Hash);
    }

    // Use the trailing comma count we calculated earlier
    let thousands_scale = trailing_comma_count;

    // Convert inline parts from placeholder indices to positions from right
    // Inline parts are stored with the number of placeholders added before the part.
    // This means the part appears before placeholder at index=placeholder_count.
    // When formatting right-to-left, placeholder at index I is at position (total-1-I) from right.
    let total_placeholders = integer_placeholders.len();
    let inline_parts = inline_parts
        .into_iter()
        .map(|(placeholder_count, part)| {
            // The part appears before placeholder[placeholder_count].
            // That placeholder is at position (total - 1 - placeholder_count) from right.
            let pos_from_right = total_placeholders - placeholder_count;
            (pos_from_right, part)
        })
        .collect();

    FormatAnalysis {
        integer_placeholders,
        decimal_placeholders,
        has_thousands_separator,
        percent_count,
        thousands_scale,
        inline_parts,
        decimal_inline_parts,
        prefix_parts,
        suffix_parts,
    }
}

/// Format a number according to a section.
pub fn format_number(
    value: f64,
    section: &Section,
    opts: &FormatOptions,
) -> Result<String, FormatError> {
    format_number_with_fill_count(value, section, opts, 0)
}

pub fn format_number_with_fill_count(
    value: f64,
    section: &Section,
    opts: &FormatOptions,
    fill_count: usize,
) -> Result<String, FormatError> {
    // Check if this is scientific notation
    let scientific_part = section.parts.iter().find_map(|p| {
        if let FormatPart::Scientific { upper, show_plus } = p {
            Some((*upper, *show_plus))
        } else {
            None
        }
    });

    if let Some((upper, show_plus)) = scientific_part {
        return format_scientific(value, section, upper, show_plus, opts, fill_count);
    }

    // Use pre-computed format type from metadata for better performance
    use crate::ast::FormatType;

    // Check if this is a fraction format
    if section.metadata.format_type == FormatType::Fraction {
        return crate::formatter::fraction::format_fraction(value, section, opts, fill_count);
    }

    // Check if this is a text-only format
    if section.metadata.format_type == FormatType::Text {
        return Ok(crate::formatter::fallback_format(value));
    }

    // Check if section has any numeric placeholders
    let has_numeric_parts = section.metadata.format_type == FormatType::Number
        || section
            .parts
            .iter()
            .any(|p| matches!(p, FormatPart::Digit(_) | FormatPart::DecimalPoint));

    // If no numeric parts, check if GeneralNumber is present
    if !has_numeric_parts {
        let has_general_number = section
            .parts
            .iter()
            .any(|p| matches!(p, FormatPart::GeneralNumber));

        if has_general_number {
            // Section has GeneralNumber part - render General in source order with surrounding parts
            // This handles cases like "General " and literals or fill directives around General
            let mut result = String::new();
            for part in &section.parts {
                match part {
                    FormatPart::GeneralNumber => {
                        // Render the General value at its source position
                        result.push_str(&crate::formatter::fallback_format(value));
                    }
                    _ => push_part(&mut result, part, fill_count),
                }
            }
            return Ok(result);
        } else {
            // No GeneralNumber - just return the literals without formatting the number
            let mut result = String::new();
            for part in &section.parts {
                push_part(&mut result, part, fill_count);
            }
            return Ok(result);
        }
    }

    let analysis = analyze_format(section);

    // Integer fast path: use integer-only arithmetic to avoid precision loss
    // Based on SSF's separate code paths in bits/66_numint.js vs bits/63_numflt.js
    // Safe integer range for f64 is < 2^53 (9007199254740992)
    //
    // Note: We only use the integer path if there are no decimal placeholders, because
    // handling optional decimal placeholders (# vs 0) requires the float path logic.
    const MAX_SAFE_INTEGER: f64 = 9007199254740992.0; // 2^53
    if value.fract() == 0.0
        && value.abs() < MAX_SAFE_INTEGER
        && analysis.decimal_placeholders.is_empty()
    {
        // Value is an exact integer within safe range and no decimal formatting needed
        return format_number_as_integer(value as i64, &analysis, opts, fill_count);
    }

    // Apply percent multiplication
    let mut adjusted_value = value.abs();
    for _ in 0..analysis.percent_count {
        adjusted_value *= 100.0;
    }

    // Apply thousands scaling (trailing commas divide by 1000 each)
    for _ in 0..analysis.thousands_scale {
        adjusted_value /= 1000.0;
    }

    // Round to the required decimal places
    // Use limited precision rounding to avoid overflow with large decimal_places
    // f64 has ~15-16 significant digits, so clamping to 15 decimal places is safe
    let decimal_places = analysis.decimal_places();
    let effective_decimal_places = decimal_places.min(15);
    let multiplier = 10_f64.powi(effective_decimal_places as i32);
    let rounded = (adjusted_value * multiplier).round() / multiplier;

    // Format the number with placeholders
    let formatted = format_with_placeholders(rounded, &analysis, opts, fill_count);

    // Build the final result with prefix and suffix
    let result = build_result(&analysis, &formatted, fill_count);

    Ok(result)
}

/// Format an integer value using integer-only arithmetic (no precision loss).
/// Based on SSF's bits/66_numint.js.
/// This path is used for values that are exact integers within safe range (< 2^53).
fn format_number_as_integer(
    value: i64,
    analysis: &FormatAnalysis,
    opts: &FormatOptions,
    fill_count: usize,
) -> Result<String, FormatError> {
    // Work with absolute value, track sign separately
    let mut adjusted_value = value.abs();

    // Apply percent multiplication (integer arithmetic)
    for _ in 0..analysis.percent_count {
        adjusted_value = adjusted_value.saturating_mul(100);
    }

    // Apply thousands scaling (integer division)
    for _ in 0..analysis.thousands_scale {
        adjusted_value /= 1000;
    }

    let formatted = format_integer(
        adjusted_value as u64,
        &analysis.integer_placeholders,
        analysis.has_thousands_separator,
        &analysis.inline_parts,
        opts,
        fill_count,
    );

    Ok(build_result(analysis, &formatted, fill_count))
}

/// Format a number according to the analysis.
fn format_with_placeholders(
    value: f64,
    analysis: &FormatAnalysis,
    opts: &FormatOptions,
    fill_count: usize,
) -> String {
    let decimal_places = analysis.decimal_places();

    // Split into integer and decimal parts
    let integer_part = value.trunc() as u64;
    let decimal_part = value.fract();

    // Format integer part
    let integer_str = format_integer(
        integer_part,
        &analysis.integer_placeholders,
        analysis.has_thousands_separator,
        &analysis.inline_parts,
        opts,
        fill_count,
    );

    // Format decimal part
    if decimal_places > 0 {
        let decimal_str = format_decimal(
            decimal_part,
            &analysis.decimal_placeholders,
            &analysis.decimal_inline_parts,
            opts,
            fill_count,
        );
        format!(
            "{}{}{}",
            integer_str, opts.locale.decimal_separator, decimal_str
        )
    } else {
        integer_str
    }
}

/// Format the integer part with placeholders and thousands separator.
fn format_integer(
    value: u64,
    placeholders: &[DigitPlaceholder],
    use_thousands: bool,
    inline_parts: &[(usize, FormatPart)],
    opts: &FormatOptions,
    fill_count: usize,
) -> String {
    let value_str = value.to_string();
    let value_digits: Vec<char> = value_str.chars().collect();

    let min_digits = placeholders.iter().filter(|p| p.is_required()).count();

    // Special case: if value is 0 and all placeholders are optional, return empty
    // BUT still include any inline literals
    if value == 0 && min_digits == 0 {
        let mut result = String::new();
        // Add any inline parts that would be in the optional placeholder region.
        // Sort by position (descending) to add them left-to-right.
        let mut sorted_parts: Vec<_> = inline_parts.iter().collect();
        sorted_parts.sort_by_key(|a| std::cmp::Reverse(a.0));

        for (_, part) in sorted_parts {
            push_part(&mut result, part, fill_count);
        }
        return result;
    }

    // SSF has different logic based on whether the format includes thousands separators
    // For formats WITHOUT thousands separators (e.g., "0#######0"):
    //   - Output length = max(value_digits, total_placeholders)
    //   - Pad using "hashq" logic: 0->'0', #->skip, ?->' '
    //   - This matches SSF's bits/66_numint.js line 69-73 for ^([#0]+)\.([#0]+)$ patterns
    // For formats WITH thousands separators (e.g., "#,###"):
    //   - Use SSF's commaify approach which formats the number then adds separators
    //   - Only show digits that fit the placeholder pattern
    let output_len = if use_thousands {
        // With thousands separators: use the narrower width to avoid spurious separators
        // This matches SSF's behavior for patterns like #{1,3},##0
        value_digits.len().max(min_digits)
    } else {
        // Without thousands separators: use the full pattern width
        value_digits.len().max(placeholders.len())
    };

    // Build right-to-left into Vec, then reverse once (O(n) instead of O(n²) with insert(0))
    // Estimate capacity: output_len + separators + inline parts.
    let separator_count = if use_thousands { output_len / 3 } else { 0 };
    let inline_chars: usize = inline_parts
        .iter()
        .map(|(_, part)| part_output_len(part, fill_count))
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
        // Position is from the right (0 = ones place, 1 = tens, etc.).
        // Push in reverse order because the complete output is reversed at the end.
        for (_, part) in inline_parts
            .iter()
            .rev()
            .filter(|(pos, _)| *pos == pos_from_right)
        {
            push_part_reversed(&mut chars, part, fill_count);
        }

        if digit_index >= 0 {
            // We have a digit from the value
            chars.push(value_digits[digit_index as usize]);
        } else {
            // No digit from value - apply SSF "hashq" padding logic
            // Use the placeholder at this position to determine padding character:
            //   0 (Zero) -> '0'
            //   # (Hash) -> nothing (skip)
            //   ? (Question) -> ' '
            let placeholder_index = placeholders.len() as isize - 1 - pos_from_right as isize;
            if placeholder_index >= 0 {
                let placeholder = placeholders[placeholder_index as usize];
                // empty_char returns Some('0') for Zero, None for Hash, Some(' ') for Question
                if let Some(c) = placeholder.empty_char() {
                    chars.push(c);
                }
                // If None (Hash), we don't push anything - this truncates the output
            }
        }
    }

    // Handle the case where we have no digits but need at least one
    if chars.is_empty() {
        // Check if we have any required placeholders
        if placeholders.iter().any(|p| p.is_required()) {
            chars.push('0');
        }
    }

    // Push any inline parts that are at positions beyond what we formatted
    // (parts in the leftmost optional placeholder region).
    for (part_pos, part) in inline_parts {
        if *part_pos >= output_len {
            push_part_reversed(&mut chars, part, fill_count);
        }
    }

    // Reverse once and collect into String
    chars.reverse();
    let result: String = chars.into_iter().collect();

    result
}

/// Format the decimal part with placeholders.
pub(super) fn format_decimal(
    value: f64,
    placeholders: &[DigitPlaceholder],
    decimal_inline_parts: &[(usize, FormatPart)],
    _opts: &FormatOptions,
    fill_count: usize,
) -> String {
    if placeholders.is_empty() {
        return String::new();
    }

    // Match SSF behavior: clamp decimal places to 10 (bits/66_numint.js line 70)
    // This avoids floating-point precision issues when multiplying by large powers of 10
    // SSF uses Math.min(r[2].length, 10) where r[2] is the decimal placeholder count
    let effective_places = placeholders.len().min(10);

    // Get the decimal digits by multiplying and truncating
    let multiplier = 10_f64.powi(effective_places as i32);
    let decimal_int = (value * multiplier).round() as u64;
    let decimal_str = format!("{:0>width$}", decimal_int, width = effective_places);
    let decimal_chars: Vec<char> = decimal_str.chars().collect();

    let mut result = String::new();

    // Check if the entire decimal part is zeros (matches SSF behavior)
    // SSF strips all trailing zeros with regex /([^0])0+$/ before applying format
    let all_zeros = decimal_chars.iter().all(|&c| c == '0');

    // Find where trailing zeros start (for # placeholders)
    // If all zeros, start from position 0 (all Hash placeholders are skipped)
    // Otherwise, scan backwards to find trailing zeros
    let mut trailing_zeros_start = if all_zeros { 0 } else { placeholders.len() };

    // Only scan within effective_places to avoid index out of bounds
    if !all_zeros {
        for i in (0..placeholders.len().min(effective_places)).rev() {
            if decimal_chars.get(i) == Some(&'0') {
                if !placeholders[i].is_required() {
                    trailing_zeros_start = i;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    // Build result, respecting placeholder rules
    for (i, placeholder) in placeholders.iter().enumerate() {
        // Insert any decimal inline parts that appear at this position.
        for (part_pos, part) in decimal_inline_parts {
            if *part_pos == i {
                push_part(&mut result, part, fill_count);
            }
        }

        // Get the character for this position
        let ch = if i < effective_places {
            decimal_chars.get(i).copied().unwrap_or('0')
        } else {
            // Beyond effective precision: apply SSF "hashq" logic
            // Hash (#) -> skip (no output)
            // Zero (0) -> '0'
            // Question (?) -> ' '
            match placeholder {
                DigitPlaceholder::Hash => {
                    // Skip - don't add anything
                    continue;
                }
                DigitPlaceholder::Zero => '0',
                DigitPlaceholder::Question => ' ',
            }
        };

        if i >= trailing_zeros_start && ch == '0' && !placeholder.is_required() {
            // Skip trailing zeros for # placeholders (only within effective_places)
            if matches!(placeholder, DigitPlaceholder::Question) {
                result.push(' ');
            }
            // For Hash, we don't add anything
        } else {
            result.push(ch);
        }
    }

    // Append any decimal inline parts that come after all placeholders.
    for (part_pos, part) in decimal_inline_parts {
        if *part_pos >= placeholders.len() {
            push_part(&mut result, part, fill_count);
        }
    }

    result
}

/// Placeholder region currently being analyzed.
#[derive(Clone, Copy)]
enum ScientificRegion {
    MantissaInteger,
    MantissaDecimal,
    Exponent,
}

/// Structural analysis of a scientific format section.
struct ScientificAnalysis {
    /// Mantissa placeholders before the decimal point.
    mantissa_integer_placeholders: Vec<DigitPlaceholder>,
    /// Mantissa placeholders after the decimal point.
    mantissa_decimal_placeholders: Vec<DigitPlaceholder>,
    /// Exponent digit placeholders.
    exponent_placeholders: Vec<DigitPlaceholder>,
    /// Parts positioned among mantissa integer placeholders.
    mantissa_integer_parts: Vec<(usize, FormatPart)>,
    /// Parts positioned among mantissa decimal placeholders.
    mantissa_decimal_parts: Vec<(usize, FormatPart)>,
    /// Parts positioned among exponent placeholders.
    exponent_parts: Vec<(usize, FormatPart)>,
}

/// Analyze a scientific format section into placeholder groups.
fn analyze_scientific_format(section: &Section) -> ScientificAnalysis {
    let active_fill = section.active_fill();
    let mut region = ScientificRegion::MantissaInteger;
    let mut mantissa_integer_placeholders = Vec::new();
    let mut mantissa_decimal_placeholders = Vec::new();
    let mut exponent_placeholders = Vec::new();
    let mut mantissa_integer_parts = Vec::new();
    let mut mantissa_decimal_parts = Vec::new();
    let mut exponent_parts = Vec::new();

    // Split placeholders and the active fill into mantissa and exponent regions.
    for (index, part) in section.parts.iter().enumerate() {
        match part {
            FormatPart::DecimalPoint if !matches!(region, ScientificRegion::Exponent) => {
                region = ScientificRegion::MantissaDecimal;
            }
            FormatPart::Scientific { .. } => {
                region = ScientificRegion::Exponent;
            }
            FormatPart::Digit(placeholder) => match region {
                ScientificRegion::MantissaInteger => {
                    mantissa_integer_placeholders.push(*placeholder)
                }
                ScientificRegion::MantissaDecimal => {
                    mantissa_decimal_placeholders.push(*placeholder)
                }
                ScientificRegion::Exponent => exponent_placeholders.push(*placeholder),
            },
            FormatPart::Fill(_) if active_fill == Some(index) => {
                match region {
                    ScientificRegion::MantissaInteger => mantissa_integer_parts
                        .push((mantissa_integer_placeholders.len(), part.clone())),
                    ScientificRegion::MantissaDecimal => mantissa_decimal_parts
                        .push((mantissa_decimal_placeholders.len(), part.clone())),
                    ScientificRegion::Exponent => {
                        exponent_parts.push((exponent_placeholders.len(), part.clone()))
                    }
                }
            }
            _ => {}
        }
    }

    // Integer-like regions are rendered right-to-left, so convert their part positions.
    let integer_part_count = mantissa_integer_placeholders.len();
    let mantissa_integer_parts = mantissa_integer_parts
        .into_iter()
        .map(|(placeholder_count, part)| (integer_part_count - placeholder_count, part))
        .collect();

    let exponent_part_count = exponent_placeholders.len();
    let exponent_parts = exponent_parts
        .into_iter()
        .map(|(placeholder_count, part)| (exponent_part_count - placeholder_count, part))
        .collect();

    ScientificAnalysis {
        mantissa_integer_placeholders,
        mantissa_decimal_placeholders,
        exponent_placeholders,
        mantissa_integer_parts,
        mantissa_decimal_parts,
        exponent_parts,
    }
}

/// Format a number in scientific notation according to a format section.
fn format_scientific(
    value: f64,
    section: &Section,
    upper: bool,
    show_plus: bool,
    opts: &FormatOptions,
    fill_count: usize,
) -> Result<String, FormatError> {
    let analysis = analyze_scientific_format(section);
    let abs_value = value.abs();
    let integer_places = analysis.mantissa_integer_placeholders.len().max(1);
    let decimal_places = analysis.mantissa_decimal_placeholders.len();

    // Calculate the exponent from the number of integer placeholders.
    // A single placeholder uses standard scientific notation, while multiple
    // placeholders group the exponent to keep more digits in the mantissa.
    let mut exponent = if abs_value == 0.0 {
        0
    } else {
        let base_exponent = abs_value.log10().floor() as i32;
        if integer_places > 1 {
            // For ##0, use exponent multiples of three and a mantissa in [1, 1000).
            ((base_exponent as f64) / (integer_places as f64)).floor() as i32
                * integer_places as i32
        } else {
            base_exponent
        }
    };

    // Convert the value to a mantissa using the selected exponent.
    let mantissa = if abs_value == 0.0 {
        0.0
    } else {
        abs_value / 10_f64.powi(exponent)
    };

    // Round the mantissa to the requested decimal precision before rendering.
    let effective_decimal_places = decimal_places.min(15);
    let multiplier = 10_f64.powi(effective_decimal_places as i32);
    let mut rounded_mantissa = (mantissa * multiplier).round() / multiplier;
    let limit = 10_f64.powi(integer_places as i32);

    // Advance the grouped exponent if rounding overflows the mantissa width.
    if rounded_mantissa >= limit {
        rounded_mantissa /= limit;
        exponent += integer_places as i32;
    }

    // Render each placeholder group separately to preserve positioned parts.
    let mantissa_integer = rounded_mantissa.trunc() as u64;
    let mantissa_decimal = rounded_mantissa.fract();
    let integer = format_integer(
        mantissa_integer,
        &analysis.mantissa_integer_placeholders,
        false,
        &analysis.mantissa_integer_parts,
        opts,
        fill_count,
    );
    let decimal = format_decimal(
        mantissa_decimal,
        &analysis.mantissa_decimal_placeholders,
        &analysis.mantissa_decimal_parts,
        opts,
        fill_count,
    );
    let exponent_digits = format_integer(
        exponent.unsigned_abs() as u64,
        &analysis.exponent_placeholders,
        false,
        &analysis.exponent_parts,
        opts,
        fill_count,
    );

    // Build the scientific result from the rendered placeholder groups.
    let mut result = String::new();

    // Apply the value sign before any mantissa prefix parts.
    if value < 0.0 {
        result.push('-');
    }
    result.push_str(&integer);
    if !analysis.mantissa_decimal_placeholders.is_empty() {
        result.push(opts.locale.decimal_separator);
        result.push_str(&decimal);
    }
    result.push(if upper { 'E' } else { 'e' });
    if exponent < 0 {
        result.push('-');
    } else if show_plus {
        result.push('+');
    }
    result.push_str(&exponent_digits);

    Ok(result)
}

/// Build the final result string with prefix and suffix parts.
fn build_result(analysis: &FormatAnalysis, formatted_number: &str, fill_count: usize) -> String {
    // Pre-allocate exact capacity (no reallocation, no waste)
    let capacity = count_part_chars(&analysis.prefix_parts, fill_count)
        + formatted_number.len()
        + count_part_chars(&analysis.suffix_parts, fill_count);
    let mut result = String::with_capacity(capacity);

    // Add prefix parts
    for part in &analysis.prefix_parts {
        push_part(&mut result, part, fill_count);
    }

    // Add the formatted number
    result.push_str(formatted_number);

    // Add suffix parts
    for part in &analysis.suffix_parts {
        push_part(&mut result, part, fill_count);
    }

    result
}

/// Calculate the output byte length for format parts (prefix/suffix).
fn count_part_chars(parts: &[FormatPart], fill_count: usize) -> usize {
    parts
        .iter()
        .map(|part| part_output_len(part, fill_count))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{LocaleCode, Section};

    fn make_section(parts: Vec<FormatPart>) -> Section {
        Section {
            condition: None,
            color: None,
            parts,
            metadata: crate::ast::SectionMetadata::default(),
        }
    }

    #[test]
    fn test_analyze_simple_integer() {
        let section = make_section(vec![FormatPart::Digit(DigitPlaceholder::Zero)]);
        let analysis = analyze_format(&section);

        assert_eq!(analysis.integer_placeholders.len(), 1);
        assert_eq!(analysis.decimal_placeholders.len(), 0);
        assert!(!analysis.has_thousands_separator);
        assert_eq!(analysis.percent_count, 0);
    }

    #[test]
    fn test_analyze_decimal_format() {
        let section = make_section(vec![
            FormatPart::Digit(DigitPlaceholder::Zero),
            FormatPart::DecimalPoint,
            FormatPart::Digit(DigitPlaceholder::Zero),
            FormatPart::Digit(DigitPlaceholder::Zero),
        ]);
        let analysis = analyze_format(&section);

        assert_eq!(analysis.integer_placeholders.len(), 1);
        assert_eq!(analysis.decimal_placeholders.len(), 2);
    }

    #[test]
    fn test_analyze_preserves_inline_parts() {
        let locale = LocaleCode {
            currency: Some("$".to_string()),
            lcid: None,
        };
        let section = make_section(vec![
            FormatPart::Digit(DigitPlaceholder::Hash),
            FormatPart::Literal("-".to_string()),
            FormatPart::Digit(DigitPlaceholder::Zero),
            FormatPart::Locale(locale.clone()),
            FormatPart::Digit(DigitPlaceholder::Zero),
            FormatPart::DecimalPoint,
            FormatPart::Digit(DigitPlaceholder::Zero),
            FormatPart::EscapedLiteral("!".to_string()),
            FormatPart::Digit(DigitPlaceholder::Zero),
        ]);

        let analysis = analyze_format(&section);

        assert_eq!(
            analysis.inline_parts,
            vec![
                (2, FormatPart::Literal("-".to_string())),
                (1, FormatPart::Locale(locale)),
            ]
        );
        assert_eq!(
            analysis.decimal_inline_parts,
            vec![(1, FormatPart::EscapedLiteral("!".to_string()))]
        );
        assert_eq!(
            format_number(123.45, &section, &FormatOptions::default()).unwrap(),
            "1-2$3.4!5"
        );
    }

    #[test]
    fn test_analyze_thousands() {
        let section = make_section(vec![
            FormatPart::Digit(DigitPlaceholder::Hash),
            FormatPart::ThousandsSeparator,
            FormatPart::Digit(DigitPlaceholder::Hash),
            FormatPart::Digit(DigitPlaceholder::Hash),
            FormatPart::Digit(DigitPlaceholder::Zero),
        ]);
        let analysis = analyze_format(&section);

        assert!(analysis.has_thousands_separator);
        assert_eq!(analysis.integer_placeholders.len(), 4);
    }

    #[test]
    fn test_analyze_percent() {
        let section = make_section(vec![
            FormatPart::Digit(DigitPlaceholder::Zero),
            FormatPart::Percent,
        ]);
        let analysis = analyze_format(&section);

        assert_eq!(analysis.percent_count, 1);
        assert_eq!(analysis.suffix_parts.len(), 1);
    }
}
