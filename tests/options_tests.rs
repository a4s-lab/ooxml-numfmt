use ooxml_numfmt::{DateSystem, FormatOptions};

#[test]
fn test_default_options() {
    let opts = FormatOptions::default();
    assert_eq!(opts.date_system, DateSystem::Date1900);
    assert_eq!(opts.fill_count, 0);
}

#[test]
fn test_fill_count_is_retained_when_options_are_cloned() {
    let options = FormatOptions {
        fill_count: 7,
        ..FormatOptions::default()
    };

    assert_eq!(options.clone().fill_count, 7);
}

#[test]
fn test_date_system_epoch() {
    assert_eq!(DateSystem::Date1900.epoch_year(), 1900);
    assert_eq!(DateSystem::Date1904.epoch_year(), 1904);
}
