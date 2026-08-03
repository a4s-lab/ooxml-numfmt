use ooxml_numfmt::ParseError;

#[test]
fn test_parse_error_display() {
    let err = ParseError::UnexpectedToken {
        position: 5,
        found: 'x',
    };
    let msg = format!("{}", err);
    assert!(msg.contains("position 5"));
    assert!(msg.contains("'x'"));
}

#[test]
fn test_parse_error_too_many_sections() {
    let err = ParseError::TooManySections;
    let msg = format!("{}", err);
    assert!(msg.contains("4"));
}

#[test]
fn test_parse_error_unsupported_format() {
    let err = ParseError::UnsupportedFormat {
        reason: "fill directives inside fraction patterns".to_string(),
    };
    let msg = err.to_string();

    assert!(msg.contains("unsupported format code"));
    assert!(msg.contains("fill directives inside fraction patterns"));
}
