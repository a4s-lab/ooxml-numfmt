use ooxml_numfmt::{format, format_default, plain_text};

#[test]
fn test_format_convenience() {
    let opts = ooxml_numfmt::FormatOptions::default();
    let parts = format(1234.5, "#,##0.00", &opts).unwrap();
    assert_eq!(plain_text(&parts), "1,234.50");
}

#[test]
fn test_format_default_convenience() {
    let parts = format_default(0.42, "0%").unwrap();
    assert_eq!(plain_text(&parts), "42%");
}

#[test]
fn test_format_invalid_code() {
    let opts = ooxml_numfmt::FormatOptions::default();
    // Empty format should error
    let result = format(42.0, "", &opts);
    assert!(result.is_err());
}
