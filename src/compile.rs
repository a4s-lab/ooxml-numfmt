//! Immutable execution-plan compilation for normalized format syntax.

use crate::ast::{
    Color, Condition, DatePart, DigitPlaceholder, FormatPart, FractionDenom, Section,
};

/// All compiled section plans owned by one number format.
#[derive(Clone)]
pub(crate) struct CompiledFormat {
    /// Plans in the same order as the public syntax sections.
    pub(crate) sections: Box<[SectionPlan]>,
}

impl CompiledFormat {
    /// Compile every retained syntax section exactly once.
    pub(crate) fn new(sections: &[Section]) -> Self {
        Self {
            sections: sections.iter().map(compile_section).collect(),
        }
    }
}

/// Value-independent execution data for one format section.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SectionPlan {
    /// Optional numeric condition used when selecting this plan.
    pub(crate) condition: Option<Condition>,
    /// Optional display color retained for compiled inspection.
    pub(crate) color: Option<Color>,
    /// Semantic dispatch category for this plan.
    pub(crate) kind: SectionKind,
    /// Ordered operations retaining source-relative layout anchors.
    pub(crate) operations: Box<[Operation]>,
    /// Date and time properties derived during compilation.
    pub(crate) date: DateSpec,
    /// Standard-number properties derived during compilation.
    pub(crate) number: Option<NumberSpec>,
    /// Scientific-notation properties derived during compilation.
    pub(crate) scientific: Option<ScientificSpec>,
    /// Fraction properties derived during compilation.
    pub(crate) fraction: Option<FractionSpec>,
}

/// Semantic dispatch categories derived from section syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SectionKind {
    /// Empty or explicit General-number syntax.
    General,
    /// Date, time, or elapsed-time syntax.
    DateTime,
    /// Standard numeric placeholder syntax.
    Number,
    /// Scientific notation syntax.
    Scientific,
    /// Fraction syntax.
    Fraction,
    /// Text-placeholder syntax.
    Text,
    /// A section containing only layout and literal operations.
    Literal,
}

/// Initial compiled operation retaining semantic order without layout expansion.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Operation {
    /// Literal or escaped text emitted as written.
    Text(Box<str>),
    /// An unresolved fill directive at its normalized syntax position.
    Fill(char),
    /// An unresolved width-skip directive.
    Skip(char),
    /// A semantic syntax part evaluated by a specialized formatter.
    Semantic(FormatPart),
}

/// Reusable numeric placeholder analysis for standard numbers and BigInt values.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NumberSpec {
    /// Integer placeholders paired with their ordered operation indices.
    pub(crate) integer_placeholders: Box<[NumberPlaceholder]>,
    /// Decimal placeholders paired with their ordered operation indices.
    pub(crate) decimal_placeholders: Box<[NumberPlaceholder]>,
    /// Operation index of the decimal separator, when present.
    pub(crate) decimal_point_index: Option<usize>,
    /// Ordered operation indices of commas that enable thousands grouping.
    pub(crate) grouping_comma_indices: Box<[usize]>,
    /// Number of trailing commas that scale the value by one thousand.
    pub(crate) thousands_scale: usize,
    /// Number of percent operations that each multiply the value by one hundred.
    pub(crate) percent_count: usize,
}

impl NumberSpec {
    /// Return the number of configured decimal placeholder positions.
    pub(crate) fn decimal_places(&self) -> usize {
        self.decimal_placeholders.len()
    }

    /// Return whether the format enables dynamic thousands grouping.
    pub(crate) fn uses_thousands(&self) -> bool {
        !self.grouping_comma_indices.is_empty()
    }
}

/// One numeric placeholder and its position in the section operation stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NumberPlaceholder {
    /// Index of the corresponding semantic operation.
    pub(crate) operation_index: usize,
    /// Placeholder behavior for missing digits.
    pub(crate) placeholder: DigitPlaceholder,
}

/// Reusable scientific-notation field positions and display policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScientificSpec {
    /// Mantissa integer placeholders in operation order.
    pub(crate) mantissa_integer: Box<[NumberPlaceholder]>,
    /// Mantissa decimal placeholders in operation order.
    pub(crate) mantissa_decimal: Box<[NumberPlaceholder]>,
    /// Operation index of the mantissa decimal point.
    pub(crate) decimal_point_index: Option<usize>,
    /// Operation index of the exponent marker.
    pub(crate) exponent_marker_index: usize,
    /// Exponent digit placeholders in operation order.
    pub(crate) exponent_digits: Box<[NumberPlaceholder]>,
    /// Whether the exponent marker uses uppercase `E`.
    pub(crate) upper: bool,
    /// Whether nonnegative exponents include an explicit plus sign.
    pub(crate) show_plus: bool,
    /// Number of percent operations applied before scientific conversion.
    pub(crate) percent_count: usize,
}

/// Reusable fraction semantics extracted from the normalized fraction node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FractionSpec {
    /// Operation index at which the semantic fraction field is emitted.
    pub(crate) operation_index: usize,
    /// Digit placeholders for a mixed fraction's integer part.
    pub(crate) integer_digits: Box<[DigitPlaceholder]>,
    /// Digit placeholders for the numerator.
    pub(crate) numerator_digits: Box<[DigitPlaceholder]>,
    /// Fixed or maximum-width denominator policy.
    pub(crate) denominator: FractionDenom,
    /// Literal spacing immediately before the slash.
    pub(crate) space_before_slash: Box<str>,
    /// Literal spacing immediately after the slash.
    pub(crate) space_after_slash: Box<str>,
}

/// Date properties that are stable for every evaluation of a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DateSpec {
    /// Whether the section uses an AM/PM marker.
    pub(crate) has_ampm: bool,
    /// Whether the section requests the alternative Hijri calendar path.
    pub(crate) is_hijri: bool,
    /// Greatest fractional-second precision in the section.
    pub(crate) max_subsecond_precision: Option<u8>,
    /// Whether the section includes elapsed-time fields.
    pub(crate) has_elapsed_time: bool,
    /// Smallest displayed time unit used for pre-rounding.
    pub(crate) smallest_time_unit: TimeUnit,
}

/// Smallest time unit displayed by a compiled date plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TimeUnit {
    /// No time components are present.
    None,
    /// Hours are the smallest displayed unit.
    Hours,
    /// Minutes are the smallest displayed unit.
    Minutes,
    /// Seconds are the smallest displayed unit.
    Seconds,
    /// Fractional seconds are displayed.
    Subseconds,
}

/// Compile one normalized syntax section into immutable execution data.
fn compile_section(section: &Section) -> SectionPlan {
    let kind = classify_section(section.parts());
    SectionPlan {
        condition: section.condition(),
        color: section.color(),
        kind,
        operations: section.parts().iter().map(compile_operation).collect(),
        date: compile_date_spec(section.parts()),
        number: (kind == SectionKind::Number).then(|| compile_number_spec(section.parts())),
        scientific: (kind == SectionKind::Scientific)
            .then(|| compile_scientific_spec(section.parts())),
        fraction: (kind == SectionKind::Fraction).then(|| compile_fraction_spec(section.parts())),
    }
}

/// Compile scientific mantissa and exponent fields from normalized syntax.
fn compile_scientific_spec(parts: &[FormatPart]) -> ScientificSpec {
    let mut mantissa_integer = Vec::new();
    let mut mantissa_decimal = Vec::new();
    let mut exponent_digits = Vec::new();
    let mut decimal_point_index = None;
    let mut exponent_marker_index = None;
    let mut upper = true;
    let mut show_plus = false;
    let mut after_decimal = false;
    let mut after_exponent = false;

    for (operation_index, part) in parts.iter().enumerate() {
        match part {
            FormatPart::Digit(placeholder) => {
                let field = NumberPlaceholder {
                    operation_index,
                    placeholder: *placeholder,
                };
                if after_exponent {
                    exponent_digits.push(field);
                } else if after_decimal {
                    mantissa_decimal.push(field);
                } else {
                    mantissa_integer.push(field);
                }
            }
            FormatPart::DecimalPoint if !after_exponent => {
                decimal_point_index = Some(operation_index);
                after_decimal = true;
            }
            FormatPart::Scientific {
                upper: part_upper,
                show_plus: part_show_plus,
            } => {
                exponent_marker_index = Some(operation_index);
                upper = *part_upper;
                show_plus = *part_show_plus;
                after_exponent = true;
            }
            _ => {}
        }
    }

    ScientificSpec {
        mantissa_integer: mantissa_integer.into_boxed_slice(),
        mantissa_decimal: mantissa_decimal.into_boxed_slice(),
        decimal_point_index,
        exponent_marker_index: exponent_marker_index
            .expect("scientific sections contain an exponent marker"),
        exponent_digits: exponent_digits.into_boxed_slice(),
        upper,
        show_plus,
        percent_count: parts
            .iter()
            .filter(|part| matches!(part, FormatPart::Percent))
            .count(),
    }
}

/// Compile the normalized fraction node into an immutable semantic spec.
fn compile_fraction_spec(parts: &[FormatPart]) -> FractionSpec {
    parts
        .iter()
        .enumerate()
        .find_map(|(operation_index, part)| {
            if let FormatPart::Fraction {
                integer_digits,
                numerator_digits,
                denominator,
                space_before_slash,
                space_after_slash,
            } = part
            {
                Some(FractionSpec {
                    operation_index,
                    integer_digits: integer_digits.clone().into_boxed_slice(),
                    numerator_digits: numerator_digits.clone().into_boxed_slice(),
                    denominator: *denominator,
                    space_before_slash: space_before_slash.clone().into_boxed_str(),
                    space_after_slash: space_after_slash.clone().into_boxed_str(),
                })
            } else {
                None
            }
        })
        .expect("fraction sections contain a normalized fraction node")
}

/// Compile standard-number placeholder behavior without a runtime value.
fn compile_number_spec(parts: &[FormatPart]) -> NumberSpec {
    let mut integer_placeholders = Vec::new();
    let mut decimal_placeholders = Vec::new();
    let mut grouping_comma_indices = Vec::new();
    let mut trailing_comma_count = 0;
    let mut decimal_point_index = None;
    let mut after_decimal = false;

    for (operation_index, part) in parts.iter().enumerate() {
        match part {
            FormatPart::Digit(placeholder) => {
                trailing_comma_count = 0;
                let compiled = NumberPlaceholder {
                    operation_index,
                    placeholder: *placeholder,
                };
                if after_decimal {
                    decimal_placeholders.push(compiled);
                } else {
                    integer_placeholders.push(compiled);
                }
            }
            FormatPart::DecimalPoint => {
                trailing_comma_count = 0;
                if decimal_point_index.is_none() {
                    decimal_point_index = Some(operation_index);
                    after_decimal = true;
                }
            }
            FormatPart::ThousandsSeparator => {
                grouping_comma_indices.push(operation_index);
                trailing_comma_count += 1;
            }
            _ => {}
        }
    }

    // Remove trailing comma indices that are not part of the grouping separator
    grouping_comma_indices.truncate(
        grouping_comma_indices
            .len()
            .saturating_sub(trailing_comma_count),
    );

    if integer_placeholders.is_empty() && decimal_point_index.is_none() {
        integer_placeholders.push(NumberPlaceholder {
            operation_index: 0,
            placeholder: DigitPlaceholder::Hash,
        });
    }

    NumberSpec {
        integer_placeholders: integer_placeholders.into_boxed_slice(),
        decimal_placeholders: decimal_placeholders.into_boxed_slice(),
        decimal_point_index,
        grouping_comma_indices: grouping_comma_indices.into_boxed_slice(),
        thousands_scale: trailing_comma_count,
        percent_count: parts
            .iter()
            .filter(|part| matches!(part, FormatPart::Percent))
            .count(),
    }
}

/// Classify a normalized section for semantic dispatch.
fn classify_section(parts: &[FormatPart]) -> SectionKind {
    if parts.iter().any(FormatPart::is_date_part) {
        SectionKind::DateTime
    } else if parts
        .iter()
        .any(|part| matches!(part, FormatPart::Fraction { .. }))
    {
        SectionKind::Fraction
    } else if parts
        .iter()
        .any(|part| matches!(part, FormatPart::Scientific { .. }))
    {
        SectionKind::Scientific
    } else if parts.iter().any(|part| {
        matches!(
            part,
            FormatPart::Digit(_) | FormatPart::DecimalPoint | FormatPart::ThousandsSeparator
        )
    }) {
        SectionKind::Number
    } else if parts
        .iter()
        .any(|part| matches!(part, FormatPart::TextPlaceholder))
    {
        SectionKind::Text
    } else if parts.is_empty()
        || parts
            .iter()
            .any(|part| matches!(part, FormatPart::GeneralNumber))
    {
        SectionKind::General
    } else {
        SectionKind::Literal
    }
}

/// Compile one syntax part while preserving every layout anchor.
fn compile_operation(part: &FormatPart) -> Operation {
    match part {
        FormatPart::Literal(text) | FormatPart::EscapedLiteral(text) => {
            Operation::Text(text.clone().into_boxed_str())
        }
        FormatPart::Fill(character) => Operation::Fill(*character),
        FormatPart::Skip(character) => Operation::Skip(*character),
        semantic => Operation::Semantic(semantic.clone()),
    }
}

/// Derive reusable date properties from normalized syntax.
fn compile_date_spec(parts: &[FormatPart]) -> DateSpec {
    let mut spec = DateSpec {
        has_ampm: false,
        is_hijri: false,
        max_subsecond_precision: None,
        has_elapsed_time: false,
        smallest_time_unit: TimeUnit::None,
    };

    for part in parts {
        match part {
            FormatPart::AmPm(_) => spec.has_ampm = true,
            FormatPart::DatePart(DatePart::BuddhistYear4Alt | DatePart::BuddhistYear2Alt) => {
                spec.is_hijri = true;
            }
            FormatPart::DatePart(DatePart::SubSecond(precision)) => {
                spec.max_subsecond_precision =
                    Some(spec.max_subsecond_precision.unwrap_or(0).max(*precision));
                spec.smallest_time_unit = TimeUnit::Subseconds;
            }
            FormatPart::DatePart(DatePart::Second | DatePart::Second2) => {
                spec.smallest_time_unit = spec.smallest_time_unit.max(TimeUnit::Seconds);
            }
            FormatPart::DatePart(DatePart::Minute | DatePart::Minute2) => {
                spec.smallest_time_unit = spec.smallest_time_unit.max(TimeUnit::Minutes);
            }
            FormatPart::DatePart(DatePart::Hour | DatePart::Hour2) => {
                spec.smallest_time_unit = spec.smallest_time_unit.max(TimeUnit::Hours);
            }
            FormatPart::Elapsed(_) => spec.has_elapsed_time = true,
            _ => {}
        }
    }

    spec
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::DigitPlaceholder;
    use crate::NumberFormat;

    /// Return the sole compiled plan for a parsed format.
    fn plan(code: &str) -> SectionPlan {
        NumberFormat::parse(code).unwrap().compiled.sections[0].clone()
    }

    #[test]
    fn classifies_primary_section_kinds() {
        assert_eq!(plan("General").kind, SectionKind::General);
        assert_eq!(plan("yyyy").kind, SectionKind::DateTime);
        assert_eq!(plan("0.00").kind, SectionKind::Number);
        assert_eq!(plan("0E+00").kind, SectionKind::Scientific);
        assert_eq!(plan("# ?/?").kind, SectionKind::Fraction);
        assert_eq!(plan("@").kind, SectionKind::Text);
        assert_eq!(plan("\"literal\"").kind, SectionKind::Literal);
    }

    #[test]
    fn preserves_the_retained_fill_anchor() {
        assert_eq!(
            plan("*A0*B").operations.as_ref(),
            &[
                Operation::Semantic(FormatPart::Digit(DigitPlaceholder::Zero)),
                Operation::Fill('B'),
            ]
        );
    }

    #[test]
    fn parsed_and_programmatic_sections_compile_equivalently() {
        let parsed = NumberFormat::parse("[Red]0").unwrap();
        let programmatic = NumberFormat::from_sections(vec![Section::new(
            None,
            Some(crate::ast::Color::Named(crate::ast::NamedColor::Red)),
            vec![FormatPart::Digit(DigitPlaceholder::Zero)],
        )]);

        assert_eq!(parsed.compiled.sections, programmatic.compiled.sections);
    }

    #[test]
    fn compiles_standard_number_semantics_once() {
        let compiled = plan("#,##0.00%,,");
        let number = compiled.number.unwrap();

        assert_eq!(number.integer_placeholders.len(), 4);
        assert_eq!(number.decimal_places(), 2);
        assert!(number.uses_thousands());
        assert_eq!(number.grouping_comma_indices.as_ref(), &[1]);
        assert_eq!(number.thousands_scale, 2);
        assert_eq!(number.percent_count, 1);
    }

    #[test]
    fn compiles_scientific_and_fraction_specs() {
        let scientific = plan("0.00E+00").scientific.unwrap();
        assert_eq!(scientific.mantissa_integer.len(), 1);
        assert_eq!(scientific.mantissa_decimal.len(), 2);
        assert_eq!(scientific.exponent_digits.len(), 2);
        assert!(scientific.upper);
        assert!(scientific.show_plus);

        let fraction = plan("# ?/?").fraction.unwrap();
        assert_eq!(fraction.integer_digits.len(), 1);
        assert_eq!(fraction.numerator_digits.len(), 1);
        assert_eq!(fraction.denominator, FractionDenom::UpToDigits(1));
    }
}
