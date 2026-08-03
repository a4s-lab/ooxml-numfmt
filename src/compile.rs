//! Immutable execution-plan compilation for normalized format syntax.

use crate::ast::{Color, Condition, DatePart, FormatPart, Section};

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
    SectionPlan {
        condition: section.condition(),
        color: section.color(),
        kind: classify_section(section.parts()),
        operations: section.parts().iter().map(compile_operation).collect(),
        date: compile_date_spec(section.parts()),
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
}
