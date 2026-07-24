# ooxml-numfmt

Excel-compatible ECMA-376 number format codes for Rust, forked from [ssfmt](https://github.com/ketbra/ssfmt).

## Features

- Parse and format Excel/OOXML number format codes
- Match Excel's actual behavior, including quirks
- Support for dates, times, percentages, fractions
- Multiple format sections (positive/negative/zero/text)
- Color and conditional format detection
- Both 1900 and 1904 date systems
- Efficient compile-once, format-many pattern

## Usage

```rust
use ooxml_numfmt::{
    ast::{Color, NamedColor},
    format_default, plain_text, FormatOptions, FormatOutput, NumberFormat,
};

// Formatting returns structured output for renderers
let parts = format_default(1234.56, "[Red]#,##0.00*.").unwrap();
assert_eq!(parts, vec![
    FormatOutput::Color(Color::Named(NamedColor::Red)),
    FormatOutput::Text("1,234.56".into()),
    FormatOutput::Fill('.'),
]);
assert_eq!(plain_text(&parts), "1,234.56");

// Compile once, format many values
let fmt = NumberFormat::parse("yyyy-mm-dd").unwrap();
let opts = FormatOptions::default();
let parts = fmt.format(46031.0, &opts);
assert_eq!(plain_text(&parts), "2026-01-09");
```

## Testing

ssfmt was inspired by SheetJS SSF, so this fork keeps its SSF-based compatibility tests. The suite covers 19.5+ million cases, with 99.9906% of evaluated cases passing.

See [docs/TESTING.md](docs/TESTING.md) for details.

## License

MIT OR Apache-2.0
