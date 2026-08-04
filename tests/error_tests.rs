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
fn test_invalid_fraction_error_display() {
    let err = ParseError::InvalidFraction {
        section_index: 2,
        reason: "fixed denominator exceeds u32::MAX",
    };
    assert_eq!(
        err.to_string(),
        "invalid fraction in section 2: fixed denominator exceeds u32::MAX"
    );
}
