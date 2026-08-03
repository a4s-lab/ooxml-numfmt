use ooxml_numfmt::{FormatOptions, NumberFormat};

#[test]
fn test_format_date_ymd() {
    let fmt = NumberFormat::parse("yyyy-mm-dd").unwrap();
    let opts = FormatOptions::default();

    // January 9, 2026 = serial 46031
    assert_eq!(fmt.format(46031.0, &opts), "2026-01-09");
}

#[test]
fn test_format_date_mdy() {
    let fmt = NumberFormat::parse("m/d/yyyy").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(fmt.format(46031.0, &opts), "1/9/2026");
}

#[test]
fn test_format_time() {
    let fmt = NumberFormat::parse("h:mm:ss").unwrap();
    let opts = FormatOptions::default();

    // 0.5 = 12:00:00 (noon)
    assert_eq!(fmt.format(0.5, &opts), "12:00:00");
}

#[test]
fn test_format_time_ampm() {
    let fmt = NumberFormat::parse("h:mm AM/PM").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(fmt.format(0.5, &opts), "12:00 PM");
    assert_eq!(fmt.format(0.25, &opts), "6:00 AM");
}

#[test]
fn test_format_datetime() {
    let fmt = NumberFormat::parse("yyyy-mm-dd h:mm").unwrap();
    let opts = FormatOptions::default();

    // January 9, 2026 at noon
    assert_eq!(fmt.format(46031.5, &opts), "2026-01-09 12:00");
}

#[test]
fn test_format_month_name() {
    let fmt = NumberFormat::parse("mmmm d, yyyy").unwrap();
    let opts = FormatOptions::default();

    assert_eq!(fmt.format(46031.0, &opts), "January 9, 2026");
}

#[test]
fn test_fill_preserves_date_part_positions() {
    let opts = FormatOptions::default();

    let prefix = NumberFormat::parse("*xyyyy-mm-dd").unwrap();
    assert_eq!(
        prefix.format_with_fill_count(46031.0, &opts, 3),
        "xxx2026-01-09"
    );

    let inline = NumberFormat::parse("yyyy*x-mm-dd").unwrap();
    assert_eq!(
        inline.format_with_fill_count(46031.0, &opts, 3),
        "2026xxx-01-09"
    );

    let suffix = NumberFormat::parse("yyyy-mm-dd*x").unwrap();
    assert_eq!(
        suffix.format_with_fill_count(46031.0, &opts, 3),
        "2026-01-09xxx"
    );
}
