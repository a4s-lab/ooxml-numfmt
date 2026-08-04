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
fn test_skip_resolution_always_be_one_space() {
    assert_eq!(format_default("yyyy_)", 46031.0), "2026 ");

    let format = NumberFormat::parse("@_)").unwrap();
    assert_eq!(
        format.format_text("text", &FormatOptions::default()),
        "text "
    );
}

#[test]
fn test_fill_number() {
    assert_eq!(format_with_fill("*x0", 42.0, 3), "xxx42");
    assert_eq!(format_with_fill("0*x", 42.0, 3), "42xxx");
    assert_eq!(format_with_fill("0*x0", 42.0, 3), "4xxx2");
    assert_eq!(format_with_fill("0.*x00", 1.23, 3), "1.xxx23");
    assert_eq!(format_with_fill("0.00%*x", 0.5, 3), "50.00%xxx");
    assert_eq!(format_with_fill("*x0", -42.0, 3), "-xxx42");
}

#[test]
fn test_fill_number_preserves_grouping_separator_side() {
    assert_eq!(format_with_fill("#*x,##0", 1234.0, 3), "1xxx,234");
    assert_eq!(format_with_fill("#,*x##0", 1234.0, 3), "1,xxx234");
    assert_eq!(format_default("#\"a\",##0", 1234.0), "1a,234");
    assert_eq!(format_default("#,\"a\"##0", 1234.0), "1,a234");
    assert_eq!(format_with_fill("#\"a\",*x##0", 1234.0, 3), "1a,xxx234");
    assert_eq!(format_with_fill("#*x,##0", 1_234_567.0, 3), "1,234xxx,567");
    assert_eq!(format_with_fill("#,*x##0", 1_234_567.0, 3), "1,234,xxx567");

    let options = FormatOptions {
        fill_count: 3,
        locale: ooxml_numfmt::Locale {
            thousands_separator: '·',
            ..ooxml_numfmt::Locale::default()
        },
        ..FormatOptions::default()
    };
    assert_eq!(
        NumberFormat::parse("#*x,##0")
            .unwrap()
            .format(1234.0, &options),
        "1xxx·234"
    );
}

#[test]
fn test_fill_number_is_omitted_by_default_or_without_directive() {
    assert_eq!(format_default("*x0", 42.0), "42");
    assert_eq!(format_with_fill("#,##0.00", 1234.5, 10), "1,234.50");
}

#[test]
fn test_fill_number_unicode_scalars() {
    assert_eq!(format_with_fill("*é0", 7.0, 0), "7");
    assert_eq!(format_with_fill("*é0", 7.0, 1), "é7");
    assert_eq!(format_with_fill("*é0", 7.0, 3), "ééé7");
}

#[test]
fn test_fill_number_only_final_directive_is_effective() {
    assert_eq!(format_with_fill("*a0*b0", 42.0, 3), "4bbb2");
}

#[test]
fn test_fill_date() {
    assert_eq!(
        format_with_fill("*xyyyy-mm-dd", 46031.0, 3),
        "xxx2026-01-09"
    );
    assert_eq!(format_with_fill("yyyy*x-mm", 46031.0, 3), "2026xxx-01");
    assert_eq!(
        format_with_fill("yyyy-mm-dd*x", 46031.0, 3),
        "2026-01-09xxx"
    );
}

#[cfg(feature = "bigint")]
#[test]
fn test_fill_bigint() {
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

    let grouped = NumberFormat::parse("#*x,##0").unwrap();
    assert_eq!(
        grouped.format_bigint(&value, &options),
        "123,456,789,012,345xxx,678"
    );
}

#[test]
fn test_fill_scientific() {
    assert_eq!(format_with_fill("*x0.00E+00", 1234.0, 3), "xxx1.23E+03");
    assert_eq!(format_with_fill("*x0.00E+00", -1234.0, 3), "-xxx1.23E+03");
    assert_eq!(format_with_fill("##0*x.0E+0", 1.0, 3), "1xxx.0E+0");
    assert_eq!(format_with_fill("0*x.0E+0", 9.96, 3), "1xxx.0E+1");
    assert_eq!(format_with_fill("0*x0.0E+0", 99.96, 3), "0xxx1.0E+2");
    assert_eq!(format_with_fill("0.*x00E+00", 1234.0, 3), "1.xxx23E+03");
    assert_eq!(format_with_fill("0.0*xE+00", 120.0, 3), "1.2xxxE+02");
    assert_eq!(format_with_fill("0.00E+0*x0", 1234.0, 3), "1.23E+0xxx3");
    assert_eq!(format_with_fill("0.00E+0*x0", 1e123, 3), "1.00E+12xxx3");
}

#[test]
fn test_fill_fraction_outer_positions_and_sign_policy() {
    assert_eq!(format_with_fill("*x# ?/?", 1.5, 3), "xxx1 1/2");
    assert_eq!(format_with_fill("*x# ?/?", -1.5, 3), "-xxx1 1/2");
    assert_eq!(format_with_fill("# ?/?*x", 1.5, 3), "1 1/2xxx");
    assert_eq!(format_with_fill("# ?/?;-*x# ?/?", -1.5, 3), "-xxx1 1/2");
    assert_eq!(format_with_fill("# ?/?;(*x# ?/?)", -1.5, 3), "(xxx1 1/2)");
}

#[test]
fn test_fill_fraction_integer_and_numerator_boundaries() {
    assert_eq!(format_with_fill("#*x# ?/?", 12.5, 3), "1xxx2 1/2");
    assert_eq!(format_with_fill("# *x?/?", 1.5, 3), "1 xxx1/2");
    assert_eq!(format_with_fill("# ?*x/?", 1.5, 3), "1 1xxx/2");
    assert_eq!(format_with_fill("# ? *x/?", 1.5, 3), "1 1 xxx/2");
}

#[test]
fn test_fill_fraction_slash_and_denominator_boundaries() {
    assert_eq!(format_with_fill("# ?/*x?", 1.5, 3), "1 1/xxx2");
    assert_eq!(format_with_fill("# ?/*x ?", 1.5, 3), "1 1/xxx 2");
    assert_eq!(format_with_fill("# ?/ *x?", 1.5, 3), "1 1/ xxx2");
    assert_eq!(format_with_fill("# ?/1*x6", 1.2, 3), "1 3/1xxx6");
}

#[test]
fn test_fill_fraction_expansion_counts() {
    for (fill_count, fill) in [(0, ""), (1, "x"), (3, "xxx")] {
        assert_eq!(
            format_with_fill("#*x# ?/?", 12.5, fill_count),
            format!("1{fill}2 1/2")
        );
        assert_eq!(
            format_with_fill("# ?/1*x6", 1.2, fill_count),
            format!("1 3/1{fill}6")
        );
    }
}

#[test]
fn test_fill_general() {
    assert_eq!(format_with_fill("*xGeneral", 42.0, 3), "xxx42");
    assert_eq!(format_with_fill("\"pre\"*xGeneral", 42.0, 3), "prexxx42");
    assert_eq!(
        format_with_fill("\"pre\"*xGeneral\"post\"", 42.0, 3),
        "prexxx42post"
    );
    assert_eq!(
        format_with_fill("\"pre\"*xGeneral\"post\"", -42.0, 3),
        "prexxx-42post"
    );
    assert_eq!(
        format_with_fill("\"pre\"General*x\"post\"", 42.0, 3),
        "pre42xxxpost"
    );
    assert_eq!(format_with_fill("General*x", 42.0, 3), "42xxx");
    assert_eq!(format_with_fill("\"[\"General*x\"]\"", 42.0, 3), "[42xxx]");
    assert_eq!(format_with_fill("\"[\"*x\"]\"", 42.0, 3), "[xxx]");
}

#[test]
fn test_fill_accounting() {
    assert_eq!(
        format_with_fill("_($* #,##0.00_)", 1234.56, 3),
        " $   1,234.56 "
    );
}

#[test]
fn test_fill_text_and_fourth_section() {
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
fn test_fill_section_selection() {
    let code = "0*x;[Red]-0*y;\"zero\"*z";

    assert_eq!(format_with_fill(code, 5.0, 2), "5xx");
    assert_eq!(format_with_fill(code, -5.0, 2), "-5yy");
    assert_eq!(format_with_fill(code, 0.0, 2), "zerozz");

    let conditional = "[>10]0*x;[<=10]0*y";
    assert_eq!(format_with_fill(conditional, 20.0, 2), "20xx");
    assert_eq!(format_with_fill(conditional, 5.0, 2), "5yy");
}

#[test]
fn test_fill_cache_reuses_formats_with_distinct_counts() {
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
fn test_fill_error_for_oversized_output() {
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
