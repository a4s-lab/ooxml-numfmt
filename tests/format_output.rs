use ooxml_numfmt::ast::{Color, NamedColor};
use ooxml_numfmt::{plain_text, FormatOptions, FormatOutput, NumberFormat};

#[test]
fn emits_section_color_before_text() {
    let format = NumberFormat::parse("[Red]0").unwrap();
    let parts = format.format(42.0, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Color(Color::Named(NamedColor::Red)),
            FormatOutput::Text("42".to_string()),
        ]
    );
}

#[test]
fn preserves_accounting_layout_directives_in_order() {
    let format = NumberFormat::parse("_($* #,##0.00_)").unwrap();
    let parts = format.format(1234.56, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Skip('('),
            FormatOutput::Text("$".to_string()),
            FormatOutput::Fill(' '),
            FormatOutput::Text("1,234.56".to_string()),
            FormatOutput::Skip(')'),
        ]
    );
    assert_eq!(plain_text(&parts), " $1,234.56 ");
}

#[test]
fn preserves_fill_after_an_integer_format() {
    let format = NumberFormat::parse("0*.").unwrap();
    let parts = format.format(42.0, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("42".to_string()),
            FormatOutput::Fill('.'),
        ]
    );
    assert_eq!(plain_text(&parts), "42");
}

#[test]
fn preserves_fill_between_scientific_mantissa_and_exponent() {
    let format = NumberFormat::parse("0.00* E+00").unwrap();
    let parts = format.format(12.34, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("1.23".to_string()),
            FormatOutput::Fill(' '),
            FormatOutput::Text("E+01".to_string()),
        ]
    );
    assert_eq!(plain_text(&parts), "1.23E+01");
}

#[test]
fn preserves_fill_inside_scientific_mantissa() {
    let format = NumberFormat::parse("0*_0.00E+00").unwrap();
    let parts = format.format(12.34, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("1".to_string()),
            FormatOutput::Fill('_'),
            FormatOutput::Text("2.34".to_string()),
            FormatOutput::Text("E+00".to_string()),
        ]
    );
    assert_eq!(plain_text(&parts), "12.34E+00");
}

#[test]
fn preserves_skip_inside_scientific_mantissa() {
    let format = NumberFormat::parse("0_-0.00E+00").unwrap();
    let parts = format.format(12.34, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("1".to_string()),
            FormatOutput::Skip('-'),
            FormatOutput::Text("2.34".to_string()),
            FormatOutput::Text("E+00".to_string()),
        ]
    );
    assert_eq!(plain_text(&parts), "1 2.34E+00");
}

#[test]
fn preserves_fill_inside_scientific_exponent() {
    let format = NumberFormat::parse("0.00E+0*_0").unwrap();
    let parts = format.format(12.34, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("1.23".to_string()),
            FormatOutput::Text("E+0".to_string()),
            FormatOutput::Fill('_'),
            FormatOutput::Text("1".to_string()),
        ]
    );
    assert_eq!(plain_text(&parts), "1.23E+01");
}

#[test]
fn preserves_skip_inside_scientific_exponent() {
    let format = NumberFormat::parse("0.00E+0_-0").unwrap();
    let parts = format.format(12.34, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("1.23".to_string()),
            FormatOutput::Text("E+0".to_string()),
            FormatOutput::Skip('-'),
            FormatOutput::Text("1".to_string()),
        ]
    );
    assert_eq!(plain_text(&parts), "1.23E+0 1");
}

#[test]
fn preserves_fill_inside_scientific_decimal_run() {
    let format = NumberFormat::parse("0.0*_0E+00").unwrap();
    let parts = format.format(12.34, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("1.2".to_string()),
            FormatOutput::Fill('_'),
            FormatOutput::Text("3".to_string()),
            FormatOutput::Text("E+01".to_string()),
        ]
    );
}

#[test]
fn keeps_negative_scientific_sign_before_inline_fill() {
    let format = NumberFormat::parse("0*_000.0E+00").unwrap();
    let parts = format.format(-12.3, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("-".to_string()),
            FormatOutput::Fill('_'),
            FormatOutput::Text("12.3".to_string()),
            FormatOutput::Text("E+00".to_string()),
        ]
    );
}

#[test]
fn keeps_negative_scientific_mantissa_coalesced() {
    let format = NumberFormat::parse("0.00E+00").unwrap();
    let parts = format.format(-12.34, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("-1.23".to_string()),
            FormatOutput::Text("E+01".to_string()),
        ]
    );
}

#[test]
fn preserves_fill_inside_zero_exponent() {
    let format = NumberFormat::parse("0.00E+0*_0").unwrap();
    let parts = format.format(0.0, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("0.00".to_string()),
            FormatOutput::Text("E+0".to_string()),
            FormatOutput::Fill('_'),
            FormatOutput::Text("0".to_string()),
        ]
    );
}

#[test]
fn preserves_literal_inside_scientific_mantissa() {
    let format = NumberFormat::parse("0\"x\"0.00E+00").unwrap();
    let parts = format.format(12.34, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("1x2.34".to_string()),
            FormatOutput::Text("E+00".to_string()),
        ]
    );
}

#[test]
fn preserves_fill_between_integer_placeholders() {
    let format = NumberFormat::parse("0*_0").unwrap();
    let parts = format.format(12.0, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("1".to_string()),
            FormatOutput::Fill('_'),
            FormatOutput::Text("2".to_string()),
        ]
    );
    assert_eq!(plain_text(&parts), "12");
}

#[test]
fn keeps_inline_fill_at_placeholder_boundary_with_extra_digits() {
    let format = NumberFormat::parse("0*_0").unwrap();
    let parts = format.format(1234.0, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("123".to_string()),
            FormatOutput::Fill('_'),
            FormatOutput::Text("4".to_string()),
        ]
    );
}

#[test]
fn preserves_fill_order_around_group_separator() {
    let options = FormatOptions::default();

    let after_comma = NumberFormat::parse("0,*_000").unwrap();
    assert_eq!(
        after_comma.format(1234.0, &options),
        vec![
            FormatOutput::Text("1,".to_string()),
            FormatOutput::Fill('_'),
            FormatOutput::Text("234".to_string()),
        ]
    );

    let before_comma = NumberFormat::parse("0*_,000").unwrap();
    assert_eq!(
        before_comma.format(1234.0, &options),
        vec![
            FormatOutput::Text("1".to_string()),
            FormatOutput::Fill('_'),
            FormatOutput::Text(",234".to_string()),
        ]
    );
}

#[test]
fn preserves_skip_between_integer_placeholders() {
    let format = NumberFormat::parse("0_-0").unwrap();
    let parts = format.format(12.0, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("1".to_string()),
            FormatOutput::Skip('-'),
            FormatOutput::Text("2".to_string()),
        ]
    );
    assert_eq!(plain_text(&parts), "1 2");
}

#[cfg(feature = "bigint")]
#[test]
fn preserves_fill_between_bigint_placeholders() {
    let format = NumberFormat::parse("0*_0").unwrap();
    let value = "12345678901234567".parse().unwrap();
    let parts = format.format_bigint(&value, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("1234567890123456".to_string()),
            FormatOutput::Fill('_'),
            FormatOutput::Text("7".to_string()),
        ]
    );
}

#[test]
fn preserves_fill_between_decimal_placeholders() {
    let format = NumberFormat::parse("0.0*_0").unwrap();
    let parts = format.format(1.23, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("1.2".to_string()),
            FormatOutput::Fill('_'),
            FormatOutput::Text("3".to_string()),
        ]
    );
    assert_eq!(plain_text(&parts), "1.23");
}

#[test]
fn preserves_fill_in_text_section() {
    let format = NumberFormat::parse("0;0;0;@*.").unwrap();
    let parts = format.format_text("abc", &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("abc".to_string()),
            FormatOutput::Fill('.'),
        ]
    );
    assert_eq!(plain_text(&parts), "abc");
}

#[test]
fn preserves_skip_in_text_section() {
    let format = NumberFormat::parse("0;0;0;@_)").unwrap();
    let parts = format.format_text("abc", &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("abc".to_string()),
            FormatOutput::Skip(')'),
        ]
    );
    assert_eq!(plain_text(&parts), "abc ");
}

#[test]
fn date_skip_reserves_width_instead_of_displaying_character() {
    let format = NumberFormat::parse("yyyy_)").unwrap();
    let parts = format.format(46031.0, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Text("2026".to_string()),
            FormatOutput::Skip(')'),
        ]
    );
    assert_eq!(plain_text(&parts), "2026 ");
}

#[test]
fn colored_general_format_keeps_color_directive() {
    let format = NumberFormat::parse("[Blue]General").unwrap();
    let parts = format.format(12.5, &FormatOptions::default());

    assert_eq!(
        parts,
        vec![
            FormatOutput::Color(Color::Named(NamedColor::Blue)),
            FormatOutput::Text("12.5".to_string()),
        ]
    );
}
