//! Characterization and end-to-end tests for OOXML fill directives.

use ooxml_numfmt::{FormatError, FormatOptions, NumberFormat};

/// Format a numeric value with default runtime options.
fn format_default(code: &str, value: f64) -> String {
    NumberFormat::parse(code)
        .unwrap()
        .format(value, &FormatOptions::default())
}

/// Format a numeric value with an explicit fill repetition count.
fn format_with_fill(code: &str, value: f64, fill_count: usize) -> String {
    let options = FormatOptions {
        fill_count,
        ..FormatOptions::default()
    };
    NumberFormat::parse(code).unwrap().format(value, &options)
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

#[test]
fn skip_resolution_is_consistent_across_date_and_text_sections() {
    assert_eq!(format_default("yyyy_)", 46031.0), "2026 ");

    let format = NumberFormat::parse("@_) ").unwrap();
    assert_eq!(
        format.format_text("text", &FormatOptions::default()),
        "text  "
    );
}

#[test]
fn general_literals_preserve_source_order() {
    assert_eq!(format_default("\"USD \"General", 42.0), "USD 42");
    assert_eq!(format_default("General\" USD\"", 42.0), "42 USD");
}

#[test]
fn expands_fill_before_after_and_inside_numbers() {
    assert_eq!(format_with_fill("*x0", 42.0, 3), "xxx42");
    assert_eq!(format_with_fill("0*x", 42.0, 3), "42xxx");
    assert_eq!(format_with_fill("0*x0", 42.0, 3), "4xxx2");
}

#[test]
fn expands_accounting_space_fill_at_its_source_position() {
    assert_eq!(
        format_with_fill("_($* #,##0.00_)", 1234.56, 3),
        " $   1,234.56 "
    );
}

#[test]
fn expands_fill_across_semantic_format_kinds() {
    assert_eq!(format_with_fill("yyyy*x-mm", 46031.0, 3), "2026xxx-01");
    assert_eq!(format_with_fill("# ?/?*x", 1.5, 3), "1 1/2xxx");
    assert_eq!(format_with_fill("0.0*xE+00", 120.0, 3), "1.2xxxE+02");
    assert_eq!(format_with_fill("\"[\"General*x\"]\"", 42.0, 3), "[42xxx]");
    assert_eq!(format_with_fill("\"[\"*x\"]\"", 42.0, 3), "[xxx]");
}

#[test]
fn expands_fill_in_text_and_fourth_sections() {
    let options = FormatOptions {
        fill_count: 3,
        ..FormatOptions::default()
    };
    let direct = NumberFormat::parse("\"[\"*x@\"]\"").unwrap();
    let fourth = NumberFormat::parse("0;0;0;\"[\"*x@\"]\"").unwrap();

    assert_eq!(direct.format_text("text", &options), "[xxxtext]");
    assert_eq!(fourth.format_text("text", &options), "[xxxtext]");
    assert_eq!(direct.format(42.0, &options), "[xxx42]");
}

#[test]
fn respects_selected_numeric_sections() {
    let code = "0*x;[Red]-0*y;\"zero\"*z";

    assert_eq!(format_with_fill(code, 5.0, 2), "5xx");
    assert_eq!(format_with_fill(code, -5.0, 2), "-5yy");
    assert_eq!(format_with_fill(code, 0.0, 2), "zerozz");

    let conditional = "[>10]0*x;[<=10]0*y";
    assert_eq!(format_with_fill(conditional, 20.0, 2), "20xx");
    assert_eq!(format_with_fill(conditional, 5.0, 2), "5yy");
}

#[test]
fn fill_count_is_exact_and_counts_unicode_scalars() {
    assert_eq!(format_with_fill("*é0", 7.0, 0), "7");
    assert_eq!(format_with_fill("*é0", 7.0, 1), "é7");
    assert_eq!(format_with_fill("*é0", 7.0, 3), "ééé7");
}

#[test]
fn non_fill_sections_ignore_nonzero_fill_count() {
    assert_eq!(format_with_fill("#,##0.00", 1234.5, 20), "1,234.50");
}

#[test]
fn only_the_final_fill_is_effective_at_its_retained_position() {
    assert_eq!(format_with_fill("*a0*b0", 42.0, 3), "4bbb2");
}

#[test]
fn cached_formats_are_reused_with_distinct_fill_counts() {
    let first = FormatOptions {
        fill_count: 1,
        ..FormatOptions::default()
    };
    let second = FormatOptions {
        fill_count: 4,
        ..FormatOptions::default()
    };

    assert_eq!(ooxml_numfmt::format(42.0, "0*x", &first).unwrap(), "42x");
    assert_eq!(
        ooxml_numfmt::format(42.0, "0*x", &second).unwrap(),
        "42xxxx"
    );
}

#[test]
fn fallible_apis_report_oversized_fill_output() {
    let options = FormatOptions {
        fill_count: usize::MAX,
        ..FormatOptions::default()
    };
    let numeric = NumberFormat::parse("*é0").unwrap();
    let text = NumberFormat::parse("*é@").unwrap();

    assert_eq!(
        numeric.try_format(42.0, &options),
        Err(FormatError::OutputTooLarge {
            fill_count: usize::MAX,
        })
    );
    assert_eq!(
        text.try_format_text("text", &options),
        Err(FormatError::OutputTooLarge {
            fill_count: usize::MAX,
        })
    );
    assert_eq!(numeric.format(42.0, &options), "42");
    assert_eq!(text.format_text("text", &options), "text");
}

#[cfg(feature = "bigint")]
#[test]
fn expands_fill_for_large_bigint_values() {
    let value = ooxml_numfmt::BigInt::parse_bytes(b"123456789012345678", 10).unwrap();
    let options = FormatOptions {
        fill_count: 3,
        ..FormatOptions::default()
    };
    let format = NumberFormat::parse("0*x0").unwrap();

    assert_eq!(
        format.format_bigint(&value, &options),
        "12345678901234567xxx8"
    );
}
