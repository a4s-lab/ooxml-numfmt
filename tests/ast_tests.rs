use ooxml_numfmt::ast::{
    Condition, DatePart, DigitPlaceholder, FormatPart, FractionPart, NamedColor, Section,
};
use ooxml_numfmt::{NumberFormat, ParseError};

#[test]
fn test_named_color_from_str() {
    assert_eq!("Red".parse::<NamedColor>().unwrap(), NamedColor::Red);
    assert_eq!("blue".parse::<NamedColor>().unwrap(), NamedColor::Blue);
    assert!("invalid".parse::<NamedColor>().is_err());
}

#[test]
fn test_condition_evaluate() {
    let cond = Condition::GreaterThan(100.0);
    assert!(cond.evaluate(150.0));
    assert!(!cond.evaluate(50.0));
    assert!(!cond.evaluate(100.0));
}

#[test]
fn test_digit_placeholder_properties() {
    assert!(DigitPlaceholder::Zero.is_required());
    assert!(!DigitPlaceholder::Hash.is_required());
    assert!(!DigitPlaceholder::Question.is_required());
}

#[test]
fn test_format_part_is_date_part() {
    let year = FormatPart::DatePart(DatePart::Year4);
    let digit = FormatPart::Digit(DigitPlaceholder::Zero);

    assert!(year.is_date_part());
    assert!(!digit.is_date_part());
}

#[test]
fn test_fraction_part_are_numeric() {
    let component = FormatPart::Fraction(FractionPart::NumeratorDigit(DigitPlaceholder::Question));

    assert!(component.is_numeric_part());
}

#[test]
fn test_number_format_is_date_format() {
    // A format with date parts should be detected as date format
    let section = Section::new(
        None,
        None,
        vec![
            FormatPart::DatePart(DatePart::Year4),
            FormatPart::Literal("-".into()),
            FormatPart::DatePart(DatePart::Month2),
        ],
    );
    let format = NumberFormat::from_sections(vec![section]).unwrap();
    assert!(format.is_date_format());
}

#[test]
fn test_number_format_sections_limit() {
    let sections: Vec<Section> = (0..5).map(|_| Section::new(None, None, vec![])).collect();
    // Should only keep first 4 sections
    let format = NumberFormat::from_sections(sections).unwrap();
    assert_eq!(format.sections().len(), 4);
}

#[test]
fn test_programmatic_fixed_denominator_overflow_returns_error() {
    let section = Section::new(
        None,
        None,
        vec![
            FormatPart::Fraction(FractionPart::NumeratorDigit(DigitPlaceholder::Question)),
            FormatPart::Fraction(FractionPart::Slash),
            FormatPart::Fraction(FractionPart::FixedDenominatorDigit(4)),
            FormatPart::Fraction(FractionPart::FixedDenominatorDigit(2)),
            FormatPart::Fraction(FractionPart::FixedDenominatorDigit(9)),
            FormatPart::Fraction(FractionPart::FixedDenominatorDigit(4)),
            FormatPart::Fraction(FractionPart::FixedDenominatorDigit(9)),
            FormatPart::Fraction(FractionPart::FixedDenominatorDigit(6)),
            FormatPart::Fraction(FractionPart::FixedDenominatorDigit(7)),
            FormatPart::Fraction(FractionPart::FixedDenominatorDigit(2)),
            FormatPart::Fraction(FractionPart::FixedDenominatorDigit(9)),
            FormatPart::Fraction(FractionPart::FixedDenominatorDigit(6)),
        ],
    );

    assert_eq!(
        NumberFormat::from_sections(vec![section]),
        Err(ParseError::InvalidFraction {
            section_index: 0,
            reason: "fixed denominator exceeds u32::MAX",
        })
    );
}

#[test]
fn test_programmatic_fraction_without_slash_is_an_ordinary_number() {
    let section = Section::new(
        None,
        None,
        vec![
            FormatPart::Fraction(FractionPart::NumeratorDigit(DigitPlaceholder::Question)),
            FormatPart::Fraction(FractionPart::DenominatorDigit(DigitPlaceholder::Question)),
        ],
    );

    let format = NumberFormat::from_sections(vec![section]).unwrap();
    assert_eq!(
        format.sections()[0].parts(),
        &[
            FormatPart::Digit(DigitPlaceholder::Question),
            FormatPart::Digit(DigitPlaceholder::Question),
        ]
    );
}

#[test]
fn test_section_constructor_normalizes_multiple_fills() {
    let section = Section::new(
        None,
        None,
        vec![
            FormatPart::Fill('a'),
            FormatPart::Digit(DigitPlaceholder::Zero),
            FormatPart::Fill('b'),
            FormatPart::Digit(DigitPlaceholder::Zero),
        ],
    );

    assert_eq!(
        section.parts(),
        &[
            FormatPart::Digit(DigitPlaceholder::Zero),
            FormatPart::Fill('b'),
            FormatPart::Digit(DigitPlaceholder::Zero),
        ]
    );
}
