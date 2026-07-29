# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.5](https://github.com/a4s-lab/ooxml-numfmt/compare/0.1.4...0.1.5) - 2026-07-29

### Added

- keep only last fill if there are more than one fills exist in a section

### Fixed

- fill should treats date tokens as literal chars

### Other

- replace select_section with select_section_index

## [0.1.4](https://github.com/a4s-lab/ooxml-numfmt/compare/0.1.3...0.1.4) - 2026-07-28

### Fixed

- General format does not switch to scientific notation when rounding crosses 1E11 #4
- limit integer fast path is only applied below 1e11, instead of 2^53

### Other

- reduce test success ratio precision
- update testing result
