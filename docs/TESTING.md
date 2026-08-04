# SheetJS SSF Test Suite Coverage

ooxml-numfmt was forked from [ketbra/ssfmt](https://github.com/ketbra/ssfmt), an implementation inspired by SheetJS SSF. The test suite includes SSF fixtures to check compatibility.

## Running Tests

Run all SSF tests:

```bash
cargo test --release ssf_ -- --nocapture
```

Run a specific test suite:

```bash
cargo test --release --test ssf_dates_tests -- --nocapture
cargo test --release --test ssf_times_tests -- --nocapture
cargo test --release --test ssf_fraction_tests -- --nocapture
cargo test --release --test ssf_oddities_tests -- --nocapture
```

## Test Suites

### JSON Test Files

1. **ssf_implied_tests.rs** - `implied.json`
   - Tests Excel's 49 built-in format IDs (0-49)
   - **Status**: 588/672 (87.5%) - 84 skipped

2. **ssf_general_tests.rs** - `general.json`
   - Tests general number formatting with various format codes
   - **Status**: 493/493 (100%) ✅

3. **ssf_fraction_tests.rs** - `fraction.json`
   - Tests fraction formatting (mixed and improper fractions)
   - **Status**: 106/106 (100%) ✅

4. **ssf_oddities_tests.rs** - `oddities.json`
   - Tests edge cases and unusual format combinations
   - **Status**: 250/275 (90.9%) - 21 skipped, 4 failing

5. **ssf_date_tests.rs** - `date.json`
   - Tests date value roundtripping
   - **Status**: Not yet implemented

6. **ssf_is_date_tests.rs** - `is_date.json`
   - Tests format string date detection
   - **Status**: Not yet implemented

### Compressed TSV Test Files

TSV fixtures are stored as `.tsv.gz` files and decompressed during tests with `flate2`.

7. **ssf_comma_tests.rs** - `comma.tsv.gz`
   - Tests thousands separators and comma divisors
   - **Status**: 105/105 (100%) ✅

8. **ssf_exp_tests.rs** - `exp.tsv.gz`
   - Tests scientific notation
   - **Status**: 177/180 (98.3%) - 3 skipped

9. **ssf_valid_tests.rs** - `valid.tsv.gz`
   - Tests format string parsing
   - **Status**: 442/442 (100%) ✅

10. **ssf_dates_tests.rs** - `dates.tsv.gz`
    - Tests date formats, including Hijri codes
    - **Status**: 3,846,024/3,846,024 (100%) ✅

11. **ssf_times_tests.rs** - `times.tsv.gz`
    - Tests time, elapsed-time, and subsecond formats
    - **Status**: 15,728,625/15,728,625 (100%) ✅

### Not Implemented

12. **cal.tsv** - Not included; disabled in the SSF test source

## Summary

- **Total test cases**: 19,576,922
- **Total passing**: 19,576,810
- **Total failing**: 4
- **Total skipped**: 108
- **Overall pass rate**: 99.99998% of evaluated cases

## Failed Cases

- **Oddities**: 4
