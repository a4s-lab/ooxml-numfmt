use ooxml_numfmt::{format_default, NumberFormat};

#[test]
fn test_general_format_parse() {
    // Should parse without error
    assert!(NumberFormat::parse("General").is_ok());
    assert!(NumberFormat::parse("GENERAL").is_ok());
    assert!(NumberFormat::parse("general").is_ok());
    assert!(NumberFormat::parse("GeNeRaL").is_ok());
}

#[test]
fn test_general_format_integers() {
    assert_eq!(format_default(0.0, "General").unwrap(), "0");
    assert_eq!(format_default(1.0, "General").unwrap(), "1");
    assert_eq!(format_default(42.0, "General").unwrap(), "42");
    assert_eq!(format_default(-42.0, "General").unwrap(), "-42");
    assert_eq!(format_default(1234567.0, "General").unwrap(), "1234567");
}

#[test]
fn test_general_format_decimals() {
    assert_eq!(format_default(1234.56, "General").unwrap(), "1234.56");
    assert_eq!(format_default(0.5, "General").unwrap(), "0.5");
    assert_eq!(format_default(0.123456, "General").unwrap(), "0.123456");
    assert_eq!(format_default(-99.99, "General").unwrap(), "-99.99");
}

#[test]
fn test_general_format_no_trailing_zeros() {
    // General format should not show unnecessary trailing zeros
    assert_eq!(format_default(1.0, "General").unwrap(), "1");
    assert_eq!(format_default(1.5, "General").unwrap(), "1.5");
    assert_eq!(format_default(1.50, "General").unwrap(), "1.5");
    assert_eq!(format_default(1.500000, "General").unwrap(), "1.5");
}

#[test]
fn test_general_format_small_numbers() {
    assert_eq!(format_default(0.1, "General").unwrap(), "0.1");
    assert_eq!(format_default(0.01, "General").unwrap(), "0.01");
    assert_eq!(format_default(0.001, "General").unwrap(), "0.001");
}

#[test]
fn test_general_format_negative_numbers() {
    assert_eq!(format_default(-1.0, "General").unwrap(), "-1");
    assert_eq!(format_default(-0.5, "General").unwrap(), "-0.5");
    assert_eq!(format_default(-1234.56, "General").unwrap(), "-1234.56");
}

#[test]
fn test_general_format_case_insensitive() {
    let value = 123.45;
    let expected = "123.45";

    assert_eq!(format_default(value, "General").unwrap(), expected);
    assert_eq!(format_default(value, "GENERAL").unwrap(), expected);
    assert_eq!(format_default(value, "general").unwrap(), expected);
    assert_eq!(format_default(value, "GeneRaL").unwrap(), expected);
}

#[test]
fn test_general_format_special_values() {
    assert_eq!(format_default(f64::NAN, "General").unwrap(), "NaN");
    assert_eq!(
        format_default(f64::INFINITY, "General").unwrap(),
        "Infinity"
    );
    assert_eq!(
        format_default(f64::NEG_INFINITY, "General").unwrap(),
        "-Infinity"
    );
}

#[test]
fn test_general_format_vs_broken_behavior() {
    // Before the fix, "General" would parse as "Gnral" (with 'e' chars consumed)
    // This test ensures we get the correct numeric output, not literal text
    let result = format_default(1234.56, "General").unwrap();

    // Should be the number formatted, not "Gnral" or similar
    assert_ne!(result, "Gnral");
    assert_ne!(result, "General");
    assert_eq!(result, "1234.56");
}

#[test]
fn test_general_format_large_integers_use_scientific() {
    assert_eq!(
        format_default(484079807176.0, "General").unwrap(),
        "4.8408E+11"
    );

    // Values around the 1E11 threshold.
    assert_eq!(format_default(100000000000.0, "General").unwrap(), "1E+11");
    assert_eq!(
        format_default(99999999999.0, "General").unwrap(),
        "99999999999"
    );
    assert_eq!(format_default(100000000001.0, "General").unwrap(), "1E+11");

    // Larger exact integers within the safe f64 range.
    assert_eq!(format_default(1000000000000.0, "General").unwrap(), "1E+12");
    assert_eq!(
        format_default(9007199254740991.0, "General").unwrap(),
        "9.0072E+15"
    ); // 2^53 - 1

    assert_eq!(
        format_default(-484079807176.0, "General").unwrap(),
        "-4.8408E+11"
    );
    assert_eq!(
        format_default(-100000000000.0, "General").unwrap(),
        "-1E+11"
    );
}

#[test]
fn test_general_format_rounding_crosses_scientific_boundary() {
    assert_eq!(
        format_default(99_999_999_999.5, "General").unwrap(),
        "1E+11"
    );
    assert_eq!(
        format_default(-99_999_999_999.5, "General").unwrap(),
        "-1E+11"
    );

    // Nearby values that do not round across the boundary stay in fixed notation.
    assert_eq!(
        format_default(99_999_999_999.4, "General").unwrap(),
        "99999999999"
    );
    assert_eq!(
        format_default(-99_999_999_999.4, "General").unwrap(),
        "-99999999999"
    );

    // Exact integers at the boundary use the same scientific notation.
    assert_eq!(
        format_default(100_000_000_000.0, "General").unwrap(),
        "1E+11"
    );
    assert_eq!(
        format_default(-100_000_000_000.0, "General").unwrap(),
        "-1E+11"
    );
}

#[test]
fn test_text_format_uses_scientific_boundary() {
    assert_eq!(format_default(484079807176.0, "@").unwrap(), "4.8408E+11");

    // Values around the 1E11 threshold.
    assert_eq!(format_default(100000000000.0, "@").unwrap(), "1E+11");
    assert_eq!(format_default(99999999999.0, "@").unwrap(), "99999999999");
    assert_eq!(format_default(100000000001.0, "@").unwrap(), "1E+11");
    assert_eq!(format_default(1000000000000.0, "@").unwrap(), "1E+12");

    // @ reevaluates notation when fixed formatting rounds across the boundary.
    assert_eq!(format_default(99_999_999_999.5, "@").unwrap(), "1E+11");
    assert_eq!(format_default(-99_999_999_999.5, "@").unwrap(), "-1E+11");

    assert_eq!(format_default(-484079807176.0, "@").unwrap(), "-4.8408E+11");
    assert_eq!(format_default(-100000000000.0, "@").unwrap(), "-1E+11");
}
