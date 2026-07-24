//! Number formatting (integers, decimals, percentages, scientific notation)

use crate::ast::{DigitPlaceholder, FormatPart, Section};
use crate::error::FormatError;
use crate::options::FormatOptions;
use crate::output::{output_for_part, push_output, text_if_nonempty, FormatOutput};

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
    /// Parts that appear inline with integer digits.
    /// Position is counted from the right (0 = after the ones place, 1 = before it, etc.).
    pub integer_inline_parts: Vec<(usize, FormatPart)>,
    /// Parts that appear inline with decimal digits.
    /// Position is counted from the left (0 = before the first decimal place).
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

/// Analyze a format section to extract its numeric structure.
pub fn analyze_format(section: &Section) -> FormatAnalysis {
    let mut integer_placeholders = Vec::new();
    let mut decimal_placeholders = Vec::new();
    let mut has_thousands_separator = false;
    let mut percent_count = 0;
    let mut integer_inline_parts = Vec::new();
    let mut decimal_inline_parts = Vec::new();
    let mut prefix_parts = Vec::new();
    let mut suffix_parts = Vec::new();
    let last_digit_index = section
        .parts
        .iter()
        .rposition(|part| matches!(part, FormatPart::Digit(_)));

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

    for (part_index, part) in section.parts.iter().enumerate() {
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
                // Only count as thousands separator if it's not a trailing comma.
                // Keep its placeholder boundary so adjacent structured parts retain
                // their source order relative to the rendered group separator.
                if commas_seen <= non_trailing_comma_count {
                    has_thousands_separator = true;
                    if seen_digit && !after_decimal {
                        integer_inline_parts.push((integer_placeholders.len(), part.clone()));
                    }
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
                    // Among decimal digits - store the boundary from the left.
                    decimal_inline_parts.push((decimal_placeholders.len(), part.clone()));
                } else {
                    // Among integer digits - convert this placeholder count below.
                    integer_inline_parts.push((integer_placeholders.len(), part.clone()));
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
            FormatPart::Fill(_) | FormatPart::Skip(_) => {
                if !seen_digit {
                    prefix_parts.push(part.clone());
                } else if last_digit_index.is_some_and(|last_digit| part_index < last_digit) {
                    if after_decimal {
                        decimal_inline_parts.push((decimal_placeholders.len(), part.clone()));
                    } else {
                        integer_inline_parts.push((integer_placeholders.len(), part.clone()));
                    }
                } else {
                    suffix_parts.push(part.clone());
                }
            }
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

    // Convert integer inline parts from placeholder counts to positions from the right.
    // A part seen after N placeholders appears before placeholder N, which is
    // `total_placeholders - N` positions from the right.
    let total_placeholders = integer_placeholders.len();
    let integer_inline_parts = integer_inline_parts
        .into_iter()
        .map(|(placeholder_count, part)| {
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
        integer_inline_parts,
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
) -> Result<Vec<FormatOutput>, FormatError> {
    // Check if this is scientific notation
    let scientific_part = section.parts.iter().find_map(|p| {
        if let FormatPart::Scientific { upper, show_plus } = p {
            Some((*upper, *show_plus))
        } else {
            None
        }
    });

    if let Some((upper, show_plus)) = scientific_part {
        return format_scientific(value, section, upper, show_plus, opts);
    }

    // Use pre-computed format type from metadata for better performance
    use crate::ast::FormatType;

    // Check if this is a fraction format
    if section.metadata.format_type == FormatType::Fraction {
        return crate::formatter::fraction::format_fraction(value, section, opts);
    }

    // Check if this is a text-only format
    if section.metadata.format_type == FormatType::Text {
        return Ok(text_if_nonempty(crate::formatter::fallback_format(value)));
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
            // Section has GeneralNumber part - use General format + append literals
            // This handles cases like "General " where we want to format the number and add a suffix
            let formatted = crate::formatter::fallback_format(value);
            let mut result = Vec::with_capacity(section.parts.len());
            for part in &section.parts {
                if matches!(part, FormatPart::GeneralNumber) {
                    result.push(FormatOutput::Text(formatted.clone()));
                } else if let Some(output) = output_for_part(part) {
                    result.push(output);
                }
            }
            return Ok(result);
        } else {
            // No GeneralNumber - return the literal and layout parts without formatting the number.
            let result = section.parts.iter().filter_map(output_for_part).collect();
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
        return format_number_as_integer(value as i64, section, opts);
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
    let formatted = format_with_placeholders(rounded, &analysis, opts);

    // Build the final result with prefix and suffix
    let result = build_result(&analysis, formatted, opts);

    Ok(result)
}

/// Format an integer value using integer-only arithmetic (no precision loss).
/// Based on SSF's bits/66_numint.js.
/// This path is used for values that are exact integers within safe range (< 2^53).
fn format_number_as_integer(
    value: i64,
    section: &Section,
    opts: &FormatOptions,
) -> Result<Vec<FormatOutput>, FormatError> {
    let analysis = analyze_format(section);

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

    // For integers, decimal places should be zero unless explicitly formatted
    let decimal_places = analysis.decimal_places();

    let mut formatted = format_integer(
        adjusted_value as u64,
        &analysis.integer_placeholders,
        analysis.has_thousands_separator,
        &analysis.integer_inline_parts,
        opts,
    );

    if decimal_places > 0 {
        push_output(
            &mut formatted,
            FormatOutput::Text(opts.locale.decimal_separator.to_string()),
        );
        for output in format_decimal(
            0.0,
            &analysis.decimal_placeholders,
            &analysis.decimal_inline_parts,
            opts,
        ) {
            push_output(&mut formatted, output);
        }
    }

    Ok(build_result(&analysis, formatted, opts))
}

/// Format a number according to the analysis.
fn format_with_placeholders(
    value: f64,
    analysis: &FormatAnalysis,
    opts: &FormatOptions,
) -> Vec<FormatOutput> {
    let decimal_places = analysis.decimal_places();

    // Split into integer and decimal parts
    let integer_part = value.trunc() as u64;
    let decimal_part = value.fract();

    // Format integer part
    let mut result = format_integer(
        integer_part,
        &analysis.integer_placeholders,
        analysis.has_thousands_separator,
        &analysis.integer_inline_parts,
        opts,
    );

    // Format decimal part
    if decimal_places > 0 {
        push_output(
            &mut result,
            FormatOutput::Text(opts.locale.decimal_separator.to_string()),
        );
        for output in format_decimal(
            decimal_part,
            &analysis.decimal_placeholders,
            &analysis.decimal_inline_parts,
            opts,
        ) {
            push_output(&mut result, output);
        }
    }

    result
}

/// Format the integer part with placeholders and thousands separator.
fn format_integer(
    value: u64,
    placeholders: &[DigitPlaceholder],
    use_thousands: bool,
    inline_parts: &[(usize, FormatPart)],
    opts: &FormatOptions,
) -> Vec<FormatOutput> {
    let value_str = value.to_string();
    let value_digits: Vec<char> = value_str.chars().collect();

    let min_digits = placeholders.iter().filter(|p| p.is_required()).count();

    // Special case: if value is 0 and all placeholders are optional, emit only
    // parts from the optional placeholder region.
    if value == 0 && min_digits == 0 {
        let mut positioned_parts: Vec<_> = inline_parts.iter().collect();
        positioned_parts.sort_by_key(|(position, _)| std::cmp::Reverse(*position));

        let mut result = Vec::new();
        for (_, part) in positioned_parts {
            if let Some(output) = output_for_part(part) {
                push_output(&mut result, output);
            }
        }
        if result.is_empty() {
            result.push(FormatOutput::Text(String::new()));
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

    format_integer_digits(
        &value_digits,
        placeholders,
        use_thousands,
        inline_parts,
        output_len,
        opts,
    )
}

/// An intermediate item used while assembling structured numeric output.
enum NumberAtom {
    Character(char),
    Output(FormatOutput),
}

pub(super) fn format_integer_digits(
    value_digits: &[char],
    placeholders: &[DigitPlaceholder],
    use_thousands: bool,
    inline_parts: &[(usize, FormatPart)],
    output_len: usize,
    opts: &FormatOptions,
) -> Vec<FormatOutput> {
    // Build right-to-left into atoms, then reverse once.
    let separator_count = if use_thousands { output_len / 3 } else { 0 };
    let estimated_capacity = output_len + separator_count + inline_parts.len();
    let mut atoms = Vec::with_capacity(estimated_capacity);

    // Process from right to left (least significant first)
    for (digit_count, pos_from_right) in (0..output_len).enumerate() {
        let digit_index = value_digits.len() as isize - 1 - pos_from_right as isize;

        let parts_at_position: Vec<_> = inline_parts
            .iter()
            .filter(|(position, _)| *position == pos_from_right)
            .map(|(_, part)| part)
            .collect();
        let needs_group_separator = use_thousands && digit_count > 0 && digit_count % 3 == 0;
        let has_explicit_group_separator = parts_at_position
            .iter()
            .any(|part| matches!(part, FormatPart::ThousandsSeparator));

        // Group separators beyond the explicit placeholder pattern are generated
        // automatically. Within the pattern, the stored comma marker preserves its
        // order relative to literals and layout directives at the same boundary.
        if needs_group_separator && !has_explicit_group_separator {
            atoms.push(NumberAtom::Character(opts.locale.thousands_separator));
        }

        // Parts at the same boundary are pushed in reverse source order because
        // the entire atom stream is reversed after right-to-left construction.
        for part in parts_at_position.into_iter().rev() {
            if matches!(part, FormatPart::ThousandsSeparator) {
                if needs_group_separator {
                    atoms.push(NumberAtom::Character(opts.locale.thousands_separator));
                }
            } else if let Some(output) = output_for_part(part) {
                atoms.push(NumberAtom::Output(output));
            }
        }

        if digit_index >= 0 {
            atoms.push(NumberAtom::Character(value_digits[digit_index as usize]));
        } else {
            let placeholder_index = placeholders.len() as isize - 1 - pos_from_right as isize;
            if placeholder_index >= 0 {
                let placeholder = placeholders[placeholder_index as usize];
                if let Some(character) = placeholder.empty_char() {
                    atoms.push(NumberAtom::Character(character));
                }
            }
        }
    }

    // Handle the case where we have no digits but need at least one
    if !atoms
        .iter()
        .any(|atom| matches!(atom, NumberAtom::Character(_)))
        && placeholders
            .iter()
            .any(|placeholder| placeholder.is_required())
    {
        atoms.push(NumberAtom::Character('0'));
    }

    // Parts beyond the formatted width belong to omitted optional placeholders.
    // Add them in reverse visual order because the atom stream is reversed below.
    let mut remaining_parts: Vec<_> = inline_parts
        .iter()
        .enumerate()
        .filter(|(_, (position, _))| *position >= output_len)
        .collect();
    remaining_parts.sort_by(
        |(left_index, (left_position, _)), (right_index, (right_position, _))| {
            left_position
                .cmp(right_position)
                .then_with(|| right_index.cmp(left_index))
        },
    );
    for (_, (_, part)) in remaining_parts {
        if let Some(output) = output_for_part(part) {
            atoms.push(NumberAtom::Output(output));
        }
    }

    atoms.reverse();

    let mut result = Vec::with_capacity(atoms.len());
    for atom in atoms {
        match atom {
            NumberAtom::Character(character) => match result.last_mut() {
                Some(FormatOutput::Text(text)) => text.push(character),
                _ => result.push(FormatOutput::Text(character.to_string())),
            },
            NumberAtom::Output(output) => push_output(&mut result, output),
        }
    }
    result
}

/// Format the decimal part with placeholders.
pub(super) fn format_decimal(
    value: f64,
    placeholders: &[DigitPlaceholder],
    inline_parts: &[(usize, FormatPart)],
    _opts: &FormatOptions,
) -> Vec<FormatOutput> {
    if placeholders.is_empty() {
        return Vec::new();
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

    let mut result = Vec::new();

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
        for (_, part) in inline_parts.iter().filter(|(position, _)| *position == i) {
            if let Some(output) = output_for_part(part) {
                push_output(&mut result, output);
            }
        }

        // Get the character for this position
        let character = if i < effective_places {
            decimal_chars.get(i).copied().unwrap_or('0')
        } else {
            // Beyond effective precision: apply SSF "hashq" logic
            // Hash (#) -> skip (no output)
            // Zero (0) -> '0'
            // Question (?) -> ' '
            match placeholder {
                DigitPlaceholder::Hash => continue,
                DigitPlaceholder::Zero => '0',
                DigitPlaceholder::Question => ' ',
            }
        };

        if i >= trailing_zeros_start && character == '0' && !placeholder.is_required() {
            if matches!(placeholder, DigitPlaceholder::Question) {
                push_output(&mut result, FormatOutput::Text(" ".to_string()));
            }
        } else {
            push_output(&mut result, FormatOutput::Text(character.to_string()));
        }
    }

    // Append parts that come after all decimal placeholders.
    for (_, part) in inline_parts
        .iter()
        .filter(|(position, _)| *position >= placeholders.len())
    {
        if let Some(output) = output_for_part(part) {
            push_output(&mut result, output);
        }
    }

    result
}

/// Build the final result with prefix and suffix parts.
fn build_result(
    analysis: &FormatAnalysis,
    formatted_number: Vec<FormatOutput>,
    _opts: &FormatOptions,
) -> Vec<FormatOutput> {
    let capacity =
        analysis.prefix_parts.len() + formatted_number.len() + analysis.suffix_parts.len();
    let mut result = Vec::with_capacity(capacity);

    result.extend(analysis.prefix_parts.iter().filter_map(output_for_part));
    result.extend(formatted_number);
    result.extend(analysis.suffix_parts.iter().filter_map(output_for_part));

    result
}

/// Format a number in scientific notation according to a format section.
fn format_scientific(
    value: f64,
    section: &Section,
    upper: bool,
    show_plus: bool,
    _opts: &FormatOptions,
) -> Result<Vec<FormatOutput>, FormatError> {
    // Count digits before and after decimal in mantissa, and exponent digits
    let mut mantissa_integer_places = 0;
    let mut mantissa_decimal_places = 0;
    let mut exponent_digits = 0;
    let mut seen_decimal = false;
    let mut after_exponent = false;

    for part in &section.parts {
        match part {
            FormatPart::Digit(_) if !seen_decimal && !after_exponent => {
                mantissa_integer_places += 1;
            }
            FormatPart::DecimalPoint if !after_exponent => {
                seen_decimal = true;
            }
            FormatPart::Digit(_) if seen_decimal && !after_exponent => {
                mantissa_decimal_places += 1;
            }
            FormatPart::Scientific { .. } => {
                after_exponent = true;
            }
            FormatPart::Digit(_) if after_exponent => {
                exponent_digits += 1;
            }
            _ => {}
        }
    }

    // Convert value to scientific notation
    let abs_value = value.abs();

    // Handle zero specially
    if abs_value == 0.0 {
        let zeros = "0".repeat(mantissa_decimal_places);
        let decimal_part = if mantissa_decimal_places > 0 {
            format!(".{}", zeros)
        } else {
            String::new()
        };
        let exp_char = if upper { 'E' } else { 'e' };
        let sign = if show_plus { "+" } else { "" };
        let mantissa = format!("0{}", decimal_part);
        let exponent_prefix = format!("{}{sign}", exp_char);

        return compose_scientific_output(section, mantissa, exponent_prefix, "00".to_string());
    }

    // Calculate exponent based on integer placeholder count
    // Standard format (0) or minimal format (no placeholder): mantissa 1-10, exponent = log10(value)
    // Format with multiple placeholders (##0): adjust exponent to use more mantissa digits
    let base_exponent = abs_value.log10().floor() as i32;

    let exponent = if mantissa_integer_places > 1 {
        // For ##0 (3 places), we want mantissa to be in range [1, 1000)
        // Adjust exponent to be a multiple of group_size to group digits
        // For ##0: exponent should be multiple of 3, giving mantissa like 123.5E+6, not 1.235E+8
        let group_size = mantissa_integer_places.max(1);
        // Use floor division to handle negative exponents correctly
        // For base_exponent = -1, group_size = 3: floor(-1/3) * 3 = -1 * 3 = -3
        ((base_exponent as f64) / (group_size as f64)).floor() as i32 * group_size
    } else {
        base_exponent
    };

    let mantissa = abs_value / 10_f64.powi(exponent);

    // Format mantissa with appropriate decimal places
    let mantissa_str = if mantissa_decimal_places > 0 {
        format!("{:.prec$}", mantissa, prec = mantissa_decimal_places)
    } else {
        format!("{:.0}", mantissa)
    };

    // Format exponent
    let exp_char = if upper { 'E' } else { 'e' };
    let exp_sign = if exponent >= 0 {
        if show_plus {
            "+"
        } else {
            ""
        }
    } else {
        "-"
    };
    let exp_abs = exponent.abs();

    // Format exponent with appropriate zero padding
    let exp_str = if exponent_digits >= 2 {
        // 0.00E+00 format uses 2-digit exponents
        format!("{:02}", exp_abs)
    } else {
        // ##0.0E+0 format uses minimal digits
        format!("{}", exp_abs)
    };
    // Apply sign for negative values.
    let mantissa = if value < 0.0 {
        format!("-{}", mantissa_str)
    } else {
        mantissa_str
    };
    let exponent_prefix = format!("{}{}", exp_char, exp_sign);

    compose_scientific_output(section, mantissa, exponent_prefix, exp_str)
}

fn compose_scientific_output(
    section: &Section,
    mantissa: String,
    exponent_prefix: String,
    exponent_digits: String,
) -> Result<Vec<FormatOutput>, FormatError> {
    let scientific_index = section
        .parts
        .iter()
        .position(|part| matches!(part, FormatPart::Scientific { .. }))
        .ok_or(FormatError::TypeMismatch {
            expected: "scientific format",
            got: "scientific format without exponent marker",
        })?;
    let mantissa_parts = &section.parts[..scientific_index];
    let exponent_parts = &section.parts[scientific_index + 1..];
    let first_mantissa_digit = mantissa_parts
        .iter()
        .position(|part| matches!(part, FormatPart::Digit(_)))
        .ok_or(FormatError::TypeMismatch {
            expected: "scientific format",
            got: "scientific format without mantissa digit placeholders",
        })?;
    let (first_exponent_digit, last_exponent_digit) =
        digit_bounds(exponent_parts).ok_or(FormatError::TypeMismatch {
            expected: "scientific format",
            got: "scientific format without exponent digit placeholders",
        })?;
    let decimal_index = mantissa_parts
        .iter()
        .position(|part| matches!(part, FormatPart::DecimalPoint));
    let integer_end = decimal_index.unwrap_or(mantissa_parts.len());
    let integer_parts = &mantissa_parts[..integer_end];
    let integer_bounds = digit_bounds(integer_parts);
    let prefix_end = integer_bounds
        .map(|(first, _)| first)
        .unwrap_or_else(|| decimal_index.unwrap_or(first_mantissa_digit));
    let (mantissa_integer, mantissa_decimal) = mantissa
        .split_once('.')
        .map_or((mantissa.as_str(), None), |(integer, decimal)| {
            (integer, Some(decimal))
        });
    let (mantissa_sign, mantissa_integer) = mantissa_integer
        .strip_prefix('-')
        .map_or((None, mantissa_integer), |integer| (Some('-'), integer));

    let mut result = Vec::new();
    result.extend(
        mantissa_parts[..prefix_end]
            .iter()
            .filter_map(output_for_part),
    );

    let mut mantissa_output = Vec::new();
    if let Some(sign) = mantissa_sign {
        push_output(&mut mantissa_output, FormatOutput::Text(sign.to_string()));
    }
    let mantissa_trailing_start;

    if let Some(decimal_index) = decimal_index {
        if let Some((first_integer_digit, last_integer_digit)) = integer_bounds {
            for output in compose_scientific_digit_run(
                &integer_parts[first_integer_digit..=last_integer_digit],
                mantissa_integer,
            ) {
                push_output(&mut mantissa_output, output);
            }
            for output in integer_parts[last_integer_digit + 1..]
                .iter()
                .filter_map(output_for_part)
            {
                push_output(&mut mantissa_output, output);
            }
        } else {
            push_output(
                &mut mantissa_output,
                FormatOutput::Text(mantissa_integer.to_string()),
            );
        }

        if let Some(decimal) = mantissa_decimal {
            push_output(&mut mantissa_output, FormatOutput::Text(".".to_string()));
            let decimal_parts = &mantissa_parts[decimal_index + 1..];
            if let Some((first_decimal_digit, last_decimal_digit)) = digit_bounds(decimal_parts) {
                for output in decimal_parts[..first_decimal_digit]
                    .iter()
                    .filter_map(output_for_part)
                {
                    push_output(&mut mantissa_output, output);
                }
                for output in compose_scientific_digit_run(
                    &decimal_parts[first_decimal_digit..=last_decimal_digit],
                    decimal,
                ) {
                    push_output(&mut mantissa_output, output);
                }
                mantissa_trailing_start = decimal_index + 1 + last_decimal_digit + 1;
            } else {
                mantissa_trailing_start = decimal_index + 1;
            }
        } else {
            mantissa_trailing_start = decimal_index;
        }
    } else {
        let (first_integer_digit, last_integer_digit) =
            integer_bounds.ok_or(FormatError::TypeMismatch {
                expected: "scientific format",
                got: "scientific format without mantissa digit placeholders",
            })?;
        for output in compose_scientific_digit_run(
            &integer_parts[first_integer_digit..=last_integer_digit],
            mantissa_integer,
        ) {
            push_output(&mut mantissa_output, output);
        }
        mantissa_trailing_start = last_integer_digit + 1;
    }

    result.extend(mantissa_output);
    result.extend(
        mantissa_parts[mantissa_trailing_start..]
            .iter()
            .filter_map(output_for_part),
    );

    let mut exponent_output = vec![FormatOutput::Text(exponent_prefix)];
    for output in exponent_parts[..first_exponent_digit]
        .iter()
        .filter_map(output_for_part)
    {
        push_output(&mut exponent_output, output);
    }
    for output in compose_scientific_digit_run(
        &exponent_parts[first_exponent_digit..=last_exponent_digit],
        &exponent_digits,
    ) {
        push_output(&mut exponent_output, output);
    }
    result.extend(exponent_output);
    result.extend(
        exponent_parts[last_exponent_digit + 1..]
            .iter()
            .filter_map(output_for_part),
    );

    Ok(result)
}

fn compose_scientific_digit_run(parts: &[FormatPart], rendered_digits: &str) -> Vec<FormatOutput> {
    let placeholder_count = parts
        .iter()
        .filter(|part| matches!(part, FormatPart::Digit(_)))
        .count();
    let digits: Vec<char> = rendered_digits.chars().collect();
    let missing = placeholder_count.saturating_sub(digits.len());
    let extra = digits.len().saturating_sub(placeholder_count);
    let mut placeholder_index = 0;
    let mut digit_index = 0;
    let mut result = Vec::new();

    for part in parts {
        if matches!(part, FormatPart::Digit(_)) {
            if placeholder_index >= missing && digit_index < digits.len() {
                let take = if placeholder_index == missing {
                    extra + 1
                } else {
                    1
                };
                let end = (digit_index + take).min(digits.len());
                let text: String = digits[digit_index..end].iter().collect();
                push_output(&mut result, FormatOutput::Text(text));
                digit_index = end;
            }
            placeholder_index += 1;
        } else if let Some(output) = output_for_part(part) {
            push_output(&mut result, output);
        }
    }

    result
}

fn digit_bounds(parts: &[FormatPart]) -> Option<(usize, usize)> {
    let first = parts
        .iter()
        .position(|part| matches!(part, FormatPart::Digit(_)))?;
    let last = parts
        .iter()
        .rposition(|part| matches!(part, FormatPart::Digit(_)))?;
    Some((first, last))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Section;

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
