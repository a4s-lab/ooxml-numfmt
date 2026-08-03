//! Compiled standard-number and scientific field evaluation.

use crate::ast::{DigitPlaceholder, FormatPart};
use crate::compile::{NumberPlaceholder, NumberSpec, ScientificSpec, SectionPlan};
use crate::error::FormatError;
use crate::options::FormatOptions;

use super::render::RenderPart;

/// Evaluate a compiled standard-number plan without resolving layout directives.
pub(super) fn evaluate_number(
    value: f64,
    plan: &SectionPlan,
    opts: &FormatOptions,
) -> Result<Vec<RenderPart>, FormatError> {
    let spec = plan.number.as_ref().ok_or(FormatError::TypeMismatch {
        expected: "compiled number format",
        got: "non-number section",
    })?;
    let fields = prepare_number_fields(value, plan.operations.len(), spec, opts);

    Ok(super::evaluate_operations(
        plan,
        |operation_index, part| match part {
            FormatPart::Digit(_) | FormatPart::DecimalPoint => fields[operation_index].clone(),
            FormatPart::ThousandsSeparator => None,
            FormatPart::Percent => Some("%".to_string()),
            FormatPart::Locale(locale) => locale.currency.clone(),
            _ => None,
        },
    ))
}

/// Evaluate a compiled scientific plan while retaining ordered layout anchors.
pub(super) fn evaluate_scientific(
    value: f64,
    plan: &SectionPlan,
    _opts: &FormatOptions,
) -> Result<Vec<RenderPart>, FormatError> {
    let spec = plan.scientific.as_ref().ok_or(FormatError::TypeMismatch {
        expected: "compiled scientific format",
        got: "non-scientific section",
    })?;
    let fields = prepare_scientific_fields(value, plan.operations.len(), spec);
    let mut output = super::evaluate_operations(plan, |operation_index, part| match part {
        FormatPart::Digit(_) | FormatPart::DecimalPoint | FormatPart::Scientific { .. } => {
            fields[operation_index].clone()
        }
        FormatPart::Percent => Some("%".to_string()),
        FormatPart::Locale(locale) => locale.currency.clone(),
        _ => None,
    });

    if value < 0.0 {
        output.insert(0, RenderPart::Text("-".to_string()));
    }

    Ok(output)
}

/// Prepare mantissa and exponent text for each scientific field operation.
fn prepare_scientific_fields(
    value: f64,
    operation_count: usize,
    spec: &ScientificSpec,
) -> Vec<Option<String>> {
    let mut adjusted = value.abs();
    for _ in 0..spec.percent_count {
        adjusted *= 100.0;
    }

    let integer_places = spec.mantissa_integer.len().max(1);
    let decimal_places = spec.mantissa_decimal.len();
    let (mantissa, exponent) = if adjusted == 0.0 {
        (0.0, 0)
    } else {
        let base_exponent = adjusted.log10().floor() as i32;
        // Multiple integer placeholders use engineering-style exponent groups.
        let exponent = if integer_places > 1 {
            ((base_exponent as f64) / (integer_places as f64)).floor() as i32
                * integer_places as i32
        } else {
            base_exponent
        };
        (adjusted / 10_f64.powi(exponent), exponent)
    };
    let mantissa_text = if decimal_places > 0 {
        format!("{mantissa:.decimal_places$}")
    } else {
        format!("{mantissa:.0}")
    };
    let (mantissa_integer, mantissa_decimal) = mantissa_text
        .split_once('.')
        .map_or((mantissa_text.as_str(), ""), |parts| parts);
    let exponent_width = if adjusted == 0.0 || spec.exponent_digits.len() >= 2 {
        2
    } else {
        1
    };
    let exponent_text = format!("{:0>width$}", exponent.abs(), width = exponent_width);
    let exponent_marker = if spec.upper { 'E' } else { 'e' };
    let exponent_sign = if exponent < 0 {
        "-"
    } else if spec.show_plus {
        "+"
    } else {
        ""
    };
    let mut fields = vec![None; operation_count];

    assign_right_aligned(mantissa_integer, &spec.mantissa_integer, &mut fields);
    assign_left_aligned(mantissa_decimal, &spec.mantissa_decimal, &mut fields);
    assign_right_aligned(&exponent_text, &spec.exponent_digits, &mut fields);
    if let Some(operation_index) = spec.decimal_point_index {
        if decimal_places > 0 {
            fields[operation_index] = Some(".".to_string());
        }
    }
    fields[spec.exponent_marker_index] = Some(format!("{exponent_marker}{exponent_sign}"));

    fields
}

/// Assign text right-to-left across placeholder operations, keeping overflow leftmost.
fn assign_right_aligned(
    text: &str,
    placeholders: &[NumberPlaceholder],
    fields: &mut [Option<String>],
) {
    if placeholders.is_empty() {
        return;
    }

    let characters: Vec<char> = text.chars().collect();
    let extra = characters.len().saturating_sub(placeholders.len());
    if extra > 0 {
        fields[placeholders[0].operation_index] = Some(characters[..extra].iter().collect());
    }

    let placeholder_start = placeholders.len().saturating_sub(characters.len());
    let character_start = characters.len().saturating_sub(placeholders.len());
    for (placeholder, character) in placeholders[placeholder_start..]
        .iter()
        .zip(&characters[character_start..])
    {
        fields[placeholder.operation_index]
            .get_or_insert_with(String::new)
            .push(*character);
    }
}

/// Assign text left-to-right across placeholder operations.
fn assign_left_aligned(
    text: &str,
    placeholders: &[NumberPlaceholder],
    fields: &mut [Option<String>],
) {
    for (placeholder, character) in placeholders.iter().zip(text.chars()) {
        fields[placeholder.operation_index] = Some(character.to_string());
    }
}

/// Prepare text for each numeric semantic operation in one value-specific pass.
fn prepare_number_fields(
    value: f64,
    operation_count: usize,
    spec: &NumberSpec,
    opts: &FormatOptions,
) -> Vec<Option<String>> {
    let mut fields = vec![None; operation_count];

    // SSF uses a separate integer path. Preserve it for exact safe integers so
    // conversion and decimal rounding cannot lose precision.
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_992.0;
    if value.fract() == 0.0
        && value.abs() < MAX_SAFE_INTEGER
        && spec.decimal_placeholders.is_empty()
    {
        let mut adjusted = (value as i64).unsigned_abs();
        for _ in 0..spec.percent_count {
            adjusted = adjusted.saturating_mul(100);
        }
        for _ in 0..spec.thousands_scale {
            adjusted /= 1000;
        }
        prepare_integer_fields(adjusted, spec, opts, &mut fields);
        return fields;
    }

    let mut adjusted = value.abs();
    for _ in 0..spec.percent_count {
        adjusted *= 100.0;
    }
    for _ in 0..spec.thousands_scale {
        adjusted /= 1000.0;
    }

    // f64 has roughly 15 significant decimal digits, so larger requested
    // precision cannot improve the rounded semantic value.
    let effective_decimal_places = spec.decimal_places().min(15);
    let multiplier = 10_f64.powi(effective_decimal_places as i32);
    let rounded = (adjusted * multiplier).round() / multiplier;
    prepare_integer_fields(rounded.trunc() as u64, spec, opts, &mut fields);
    prepare_decimal_fields(rounded.fract(), spec, &mut fields);

    if let Some(operation_index) = spec.decimal_point_index {
        fields[operation_index] = Some(opts.locale.decimal_separator.to_string());
    }

    fields
}

/// Map integer digits, padding, and grouping to their source placeholders.
fn prepare_integer_fields(
    value: u64,
    spec: &NumberSpec,
    opts: &FormatOptions,
    fields: &mut [Option<String>],
) {
    let value_digits: Vec<char> = value.to_string().chars().collect();
    prepare_integer_digit_fields(&value_digits, spec, opts, fields);
}

/// Map an arbitrary-length integer digit slice to compiled placeholders.
fn prepare_integer_digit_fields(
    value_digits: &[char],
    spec: &NumberSpec,
    opts: &FormatOptions,
    fields: &mut [Option<String>],
) {
    let placeholders = &spec.integer_placeholders;
    if placeholders.is_empty() {
        return;
    }

    let minimum_digits = placeholders
        .iter()
        .filter(|field| field.placeholder.is_required())
        .count();
    if value_digits.iter().all(|character| *character == '0') && minimum_digits == 0 {
        return;
    }

    // Without grouping SSF evaluates the full placeholder width. With grouping
    // it narrows optional leading placeholders to the displayed digit width.
    let output_len = if spec.use_thousands {
        value_digits.len().max(minimum_digits)
    } else {
        value_digits.len().max(placeholders.len())
    };
    let extra_digits = output_len.saturating_sub(placeholders.len());
    let active_placeholders = output_len.min(placeholders.len());
    let first_placeholder = placeholders.len() - active_placeholders;

    for logical_index in 0..output_len {
        let placeholder_index = if logical_index < extra_digits {
            0
        } else {
            first_placeholder + logical_index - extra_digits
        };
        let placeholder = placeholders[placeholder_index];
        let value_index =
            value_digits.len() as isize - output_len as isize + logical_index as isize;
        let output = fields[placeholder.operation_index].get_or_insert_with(String::new);

        if value_index >= 0 {
            output.push(value_digits[value_index as usize]);
        } else if let Some(character) = placeholder.placeholder.empty_char() {
            output.push(character);
        }

        let remaining_positions = output_len - logical_index - 1;
        if spec.use_thousands && remaining_positions > 0 && remaining_positions % 3 == 0 {
            output.push(opts.locale.thousands_separator);
        }
    }
}

/// Evaluate exact arbitrary-length integer digits through a compiled number plan.
#[cfg(feature = "bigint")]
pub(super) fn evaluate_integer_digits(
    digits: &str,
    plan: &SectionPlan,
    opts: &FormatOptions,
) -> Result<Vec<RenderPart>, FormatError> {
    let spec = plan.number.as_ref().ok_or(FormatError::TypeMismatch {
        expected: "compiled number format",
        got: "non-number section",
    })?;
    let mut fields = vec![None; plan.operations.len()];
    let value_digits: Vec<char> = digits.chars().collect();
    prepare_integer_digit_fields(&value_digits, spec, opts, &mut fields);
    prepare_decimal_fields(0.0, spec, &mut fields);
    if let Some(operation_index) = spec.decimal_point_index {
        fields[operation_index] = Some(opts.locale.decimal_separator.to_string());
    }

    Ok(super::evaluate_operations(
        plan,
        |operation_index, part| match part {
            FormatPart::Digit(_) | FormatPart::DecimalPoint => fields[operation_index].clone(),
            FormatPart::ThousandsSeparator => None,
            FormatPart::Percent => Some("%".to_string()),
            FormatPart::Locale(locale) => locale.currency.clone(),
            _ => None,
        },
    ))
}

/// Map rounded fractional digits and optional padding to decimal placeholders.
fn prepare_decimal_fields(value: f64, spec: &NumberSpec, fields: &mut [Option<String>]) {
    let placeholders = &spec.decimal_placeholders;
    if placeholders.is_empty() {
        return;
    }

    // SSF evaluates at most ten fractional digits before applying placeholder
    // padding beyond that precision.
    let effective_places = placeholders.len().min(10);
    let multiplier = 10_f64.powi(effective_places as i32);
    let decimal_integer = (value * multiplier).round() as u64;
    let decimal_text = format!("{decimal_integer:0>effective_places$}");
    let decimal_digits: Vec<char> = decimal_text.chars().collect();
    let all_zeros = decimal_digits.iter().all(|character| *character == '0');
    let mut trailing_zeros_start = if all_zeros { 0 } else { placeholders.len() };

    if !all_zeros {
        for index in (0..placeholders.len().min(effective_places)).rev() {
            if decimal_digits.get(index) == Some(&'0') {
                if !placeholders[index].placeholder.is_required() {
                    trailing_zeros_start = index;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    for (index, field) in placeholders.iter().enumerate() {
        let character = if index < effective_places {
            decimal_digits.get(index).copied().unwrap_or('0')
        } else {
            match field.placeholder.empty_char() {
                Some(character) => character,
                None => continue,
            }
        };

        if index >= trailing_zeros_start && character == '0' && !field.placeholder.is_required() {
            if matches!(field.placeholder, DigitPlaceholder::Question) {
                fields[field.operation_index] = Some(" ".to_string());
            }
        } else {
            fields[field.operation_index] = Some(character.to_string());
        }
    }
}

/// Format a simple integer value with digit placeholders.
///
/// This follows SSF's `write_num` helper and is shared by fraction fields.
pub(crate) fn format_simple_with_placeholders(
    value: u64,
    placeholders: &[DigitPlaceholder],
) -> String {
    if placeholders.is_empty() {
        return value.to_string();
    }

    let value_text = value.to_string();
    let value_digits: Vec<char> = value_text.chars().collect();
    if value_digits.len() > placeholders.len() {
        return value_text;
    }

    let mut characters = Vec::with_capacity(placeholders.len());
    for position_from_right in 0..placeholders.len() {
        let digit_index = value_digits.len() as isize - 1 - position_from_right as isize;
        let placeholder_index = placeholders.len() - 1 - position_from_right;
        if digit_index >= 0 {
            characters.push(value_digits[digit_index as usize]);
        } else if let Some(character) = placeholders[placeholder_index].empty_char() {
            characters.push(character);
        }
    }

    characters.reverse();
    characters.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_fill_inside_numeric_placeholders() {
        let format = crate::NumberFormat::parse("0*-0").unwrap();
        let plan = &format.compiled.sections[0];
        let parts = evaluate_number(42.0, plan, &FormatOptions::default()).unwrap();

        assert_eq!(
            super::super::render::resolve_layout(&parts, 3).unwrap(),
            "4---2"
        );
    }

    #[test]
    fn preserves_fill_inside_scientific_fields() {
        let format = crate::NumberFormat::parse("0.0*xE+00").unwrap();
        let plan = &format.compiled.sections[0];
        let parts = evaluate_scientific(120.0, plan, &FormatOptions::default()).unwrap();

        assert_eq!(
            super::super::render::resolve_layout(&parts, 3).unwrap(),
            "1.2xxxE+02"
        );
    }
}
