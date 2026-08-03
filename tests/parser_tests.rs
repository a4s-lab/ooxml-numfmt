//! Tests for the format code parser.

use ooxml_numfmt::ast::{Color, DatePart, DigitPlaceholder, FormatPart, NamedColor};
use ooxml_numfmt::NumberFormat;

#[test]
fn test_parse_simple_number() {
    let fmt = NumberFormat::parse("#,##0.00").unwrap();
    assert_eq!(fmt.sections().len(), 1);
    assert!(!fmt.is_date_format());
}

#[test]
fn test_parse_date_format() {
    let fmt = NumberFormat::parse("yyyy-mm-dd").unwrap();
    assert_eq!(fmt.sections().len(), 1);
    assert!(fmt.is_date_format());
}

#[test]
fn test_parse_multiple_sections() {
    let fmt = NumberFormat::parse("#,##0;-#,##0;0").unwrap();
    assert_eq!(fmt.sections().len(), 3);
}

#[test]
fn test_parse_color() {
    let fmt = NumberFormat::parse("[Red]0").unwrap();
    assert!(fmt.has_color());
    assert_eq!(
        fmt.sections()[0].color(),
        Some(Color::Named(NamedColor::Red))
    );
}

#[test]
fn test_parse_percentage() {
    let fmt = NumberFormat::parse("0%").unwrap();
    assert!(fmt.is_percentage());
}

#[test]
fn test_parse_text_format() {
    let fmt = NumberFormat::parse("@").unwrap();
    assert!(fmt.is_text_format());
}

#[test]
fn test_parse_too_many_sections() {
    let result = NumberFormat::parse("0;0;0;0;0");
    // Should succeed but truncate to 4 sections
    let fmt = result.unwrap();
    assert_eq!(fmt.sections().len(), 4);
}

#[test]
fn test_minute_vs_month_disambiguation() {
    // In "mm-dd" without hour, m is month
    let fmt = NumberFormat::parse("mm-dd").unwrap();
    let parts = fmt.sections()[0].parts();
    assert!(matches!(parts[0], FormatPart::DatePart(DatePart::Month2)));

    // In "hh:mm" with hour before, m is minute
    let fmt = NumberFormat::parse("hh:mm").unwrap();
    let parts = fmt.sections()[0].parts();
    // Find the minute part after the hour
    let has_minute = parts
        .iter()
        .any(|p| matches!(p, FormatPart::DatePart(DatePart::Minute2)));
    assert!(has_minute, "Expected Minute2 after hour");
}

#[test]
fn test_fill_treats_date_tokens_as_literal_characters() {
    for fill in [
        'B', 'b', 'y', 'Y', 'm', 'M', 'd', 'D', 'h', 'H', 's', 'S', 'E', 'e',
    ] {
        let format_code = format!("*{fill}");
        let fmt = NumberFormat::parse(&format_code).unwrap();

        assert_eq!(fmt.sections()[0].parts(), &[FormatPart::Fill(fill)]);
    }
}

#[test]
fn test_parse_multiple_fill_operators_keeps_only_last() {
    let fmt = NumberFormat::parse("*A0*B").unwrap();

    assert_eq!(
        fmt.sections()[0].parts(),
        &[
            FormatPart::Digit(DigitPlaceholder::Zero),
            FormatPart::Fill('B'),
        ]
    );
}
