//! Characterization and end-to-end tests for OOXML fill directives.

use ooxml_numfmt::{FormatOptions, NumberFormat};

/// Format a numeric value with default runtime options.
fn format_default(code: &str, value: f64) -> String {
    NumberFormat::parse(code)
        .unwrap()
        .format(value, &FormatOptions::default())
}

#[test]
fn default_numeric_formatting_omits_fill_output() {
    assert_eq!(format_default("*x0", 42.0), "42");
}

#[test]
fn default_date_formatting_omits_fill_output() {
    assert_eq!(format_default("yyyy*-mm", 46031.0), "202601");
}

#[test]
fn default_text_formatting_omits_fill_output() {
    let format = NumberFormat::parse("\"<\"*x@\">\"").unwrap();

    assert_eq!(
        format.format_text("text", &FormatOptions::default()),
        "<text>"
    );
}

#[test]
fn default_accounting_formatting_preserves_non_fill_content() {
    assert_eq!(format_default("_($* #,##0.00_)", 1234.56), " $1,234.56 ");
}

#[test]
fn default_formatting_uses_only_the_retained_final_fill() {
    let format = NumberFormat::parse("*A0*B").unwrap();

    assert_eq!(format.format(42.0, &FormatOptions::default()), "42");
    assert_eq!(
        format.sections()[0].parts(),
        &[
            ooxml_numfmt::ast::FormatPart::Digit(ooxml_numfmt::ast::DigitPlaceholder::Zero,),
            ooxml_numfmt::ast::FormatPart::Fill('B'),
        ]
    );
}
