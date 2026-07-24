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
