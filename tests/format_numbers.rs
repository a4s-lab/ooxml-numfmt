use ooxml_numfmt::{FormatOptions, NumberFormat};

#[test]
fn test_format_integer() {
    let fmt = NumberFormat::parse("0").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(fmt.format(42.0, &opts), "42");
    assert_eq!(fmt.format(42.7, &opts), "43"); // Rounds
}

#[test]
fn test_format_decimal() {
    let fmt = NumberFormat::parse("0.00").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(fmt.format(42.0, &opts), "42.00");
    assert_eq!(fmt.format(42.567, &opts), "42.57");
}

#[test]
fn test_format_thousands() {
    let fmt = NumberFormat::parse("#,##0").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(fmt.format(1234567.0, &opts), "1,234,567");
    assert_eq!(fmt.format(123.0, &opts), "123");
}

#[test]
fn test_format_percentage() {
    let fmt = NumberFormat::parse("0%").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(fmt.format(0.42, &opts), "42%");
    assert_eq!(fmt.format(1.5, &opts), "150%");
}

#[test]
fn test_format_hash_placeholder() {
    let fmt = NumberFormat::parse("#.##").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(fmt.format(42.5, &opts), "42.5");
    assert_eq!(fmt.format(42.0, &opts), "42.");
}

#[test]
fn test_format_negative_section() {
    let fmt = NumberFormat::parse("0;-0").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(fmt.format(42.0, &opts), "42");
    assert_eq!(fmt.format(-42.0, &opts), "-42");
}

#[test]
fn test_fill_repeats_at_numeric_source_position() {
    let opts = FormatOptions::default();

    let prefix = NumberFormat::parse("*x0").unwrap();
    assert_eq!(prefix.format_with_fill_count(42.0, &opts, 3), "xxx42");

    let inline = NumberFormat::parse("0*x0").unwrap();
    assert_eq!(inline.format_with_fill_count(42.0, &opts, 3), "4xxx2");

    let after_decimal = NumberFormat::parse("0.*x00").unwrap();
    assert_eq!(
        after_decimal.format_with_fill_count(1.23, &opts, 3),
        "1.xxx23"
    );

    let suffix = NumberFormat::parse("0*x").unwrap();
    assert_eq!(suffix.format_with_fill_count(42.0, &opts, 3), "42xxx");

    let after_percent = NumberFormat::parse("0.00%*x").unwrap();
    assert_eq!(
        after_percent.format_with_fill_count(0.5, &opts, 3),
        "50.00%xxx"
    );
}

#[test]
fn test_scientific_fill_uses_rendered_placeholder_positions() {
    let opts = FormatOptions::default();

    let prefix = NumberFormat::parse("*x0.00E+00").unwrap();
    assert_eq!(
        prefix.format_with_fill_count(1234.0, &opts, 3),
        "xxx1.23E+03"
    );
    assert_eq!(
        prefix.format_with_fill_count(-1234.0, &opts, 3),
        "-xxx1.23E+03"
    );

    let before_decimal = NumberFormat::parse("##0*x.0E+0").unwrap();
    assert_eq!(
        before_decimal.format_with_fill_count(1.0, &opts, 3),
        "1xxx.0E+0"
    );

    let rounded_mantissa = NumberFormat::parse("0*x.0E+0").unwrap();
    assert_eq!(
        rounded_mantissa.format_with_fill_count(9.96, &opts, 3),
        "1xxx.0E+1"
    );

    let overflowing_mantissa = NumberFormat::parse("0*x0.0E+0").unwrap();
    assert_eq!(
        overflowing_mantissa.format_with_fill_count(99.96, &opts, 3),
        "0xxx1.0E+2"
    );

    let after_decimal = NumberFormat::parse("0.*x00E+00").unwrap();
    assert_eq!(
        after_decimal.format_with_fill_count(1234.0, &opts, 3),
        "1.xxx23E+03"
    );

    let exponent = NumberFormat::parse("0.00E+0*x0").unwrap();
    assert_eq!(
        exponent.format_with_fill_count(1234.0, &opts, 3),
        "1.23E+0xxx3"
    );
    assert_eq!(
        exponent.format_with_fill_count(1e123, &opts, 3),
        "1.00E+12xxx3"
    );
}

#[test]
fn test_fraction_fill_preserves_outer_positions() {
    let opts = FormatOptions::default();

    let prefix = NumberFormat::parse("*x# ?/?").unwrap();
    assert_eq!(prefix.format_with_fill_count(1.5, &opts, 3), "xxx1 1/2");
    assert_eq!(prefix.format_with_fill_count(-1.5, &opts, 3), "-xxx1 1/2");

    let suffix = NumberFormat::parse("# ?/?*x").unwrap();
    assert_eq!(suffix.format_with_fill_count(1.5, &opts, 3), "1 1/2xxx");
}

#[test]
fn test_fill_follows_implicit_negative_sign() {
    let format = NumberFormat::parse("*x0").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(format.format_with_fill_count(-42.0, &opts, 3), "-xxx42");
}

#[test]
fn test_fill_count_is_noop_when_zero_or_fill_is_absent() {
    let opts = FormatOptions::default();

    let with_fill = NumberFormat::parse("*x0").unwrap();
    assert_eq!(with_fill.format_with_fill_count(42.0, &opts, 0), "42");
    assert_eq!(with_fill.format(42.0, &opts), "42");

    let without_fill = NumberFormat::parse("#,##0.00").unwrap();
    assert_eq!(
        without_fill.format_with_fill_count(1234.5, &opts, 10),
        "1,234.50"
    );
}

#[test]
fn test_fill_uses_selected_numeric_section() {
    let format = NumberFormat::parse("*p0;*n0;*z0").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(format.format_with_fill_count(42.0, &opts, 2), "pp42");
    assert_eq!(format.format_with_fill_count(-42.0, &opts, 2), "nn42");
    assert_eq!(format.format_with_fill_count(0.0, &opts, 2), "zz0");
}

#[test]
fn test_fill_repeats_unicode_character() {
    let format = NumberFormat::parse("*한0").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(format.format_with_fill_count(7.0, &opts, 3), "한한한7");
}

#[test]
fn test_fill_applies_to_literal_only_section() {
    let format = NumberFormat::parse("*x\"value\"").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(format.format_with_fill_count(42.0, &opts, 3), "xxxvalue");
}
