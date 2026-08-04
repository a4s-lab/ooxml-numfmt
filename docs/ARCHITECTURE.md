# Architecture

This document records the durable architectural principles of `ooxml-numfmt`.
It names the types that define important boundaries, but intentionally avoids
their private field layouts and algorithm details. The type names are landmarks
for contributors, not promises that private representations will never evolve.

## Goals

The crate aims to provide spreadsheet-compatible number formatting that is:

- reusable across many values;
- deterministic for a given format, value, and set of options;
- safe for untrusted format codes and runtime inputs;
- extensible without coupling unrelated formatting domains;
- compatible with existing public behavior.

## Processing model

```mermaid
flowchart LR
    A["Format code"] --> B["Section and FormatPart"]
    B --> C["NumberFormat with CompiledFormat"]
    C --> D["Select section"]
    D --> E["Evaluate value semantics"]
    E --> F["RenderPart sequence"]
    F --> G["Resolve layout"]
    G --> H["Formatted text"]
```

Each stage has one primary responsibility. Data flows forward through the
pipeline; later stages must not reinterpret or mutate the syntax owned by
earlier stages.

## Core architectural types

These types are important because they establish ownership or a boundary
between stages. Their private fields and helper types are not architectural
contracts.

| Type                               | Role                                                        | Stability expectation                                         |
| ---------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------- |
| `NumberFormat`                     | Public, reusable representation of one format code          | Public behavior is stable; internal storage may change        |
| `Section` and `FormatPart`         | Public normalized syntax and source ordering                | Syntax meaning and ordering are stable                        |
| `CompiledFormat` and `SectionPlan` | Private, immutable execution plans derived from syntax      | The compile/execute boundary is stable; plan shape may change |
| `Operation`                        | Private ordered bridge between parsed syntax and evaluators | Source-order preservation is stable; variants may change      |
| `RenderPart`                       | Private boundary between semantic evaluation and layout     | Deferred-layout boundary is stable; representation may change |
| `FormatOptions`                    | Public per-call formatting and layout policy                | Public option semantics are stable                            |
| `FormatError`                      | Public failures from fallible formatting APIs               | Public error behavior follows normal compatibility policy     |

Domain-specific compiled specifications, such as the current number, date,
scientific, and fraction plans, are intentionally omitted from this table. They
are useful implementation structures, but do not define a cross-domain
architectural boundary.

## Architectural principles

### Parse once, format many times

Parsing and other value-independent analysis happen when a `NumberFormat` is
created. The resulting format is immutable and reusable. Formatting a value
should not reparse the format code or repeat analysis that can be derived in
advance.

This separation is also the basis of caching: a cached format must remain valid
for any value and any runtime options.

### Keep syntax and execution concerns separate

`Section` and `FormatPart` preserve the meaning and ordering of the format code.
`CompiledFormat` and its `SectionPlan` values derive information needed for
efficient formatting, but remain internal execution details.

These representations must never drift apart. Construction is the boundary at
which syntax is validated, normalized, and prepared for execution. After that
boundary, neither representation is mutable.

Public equality, inspection, and debugging of `NumberFormat` describe the
format code rather than the internal execution strategy.

### Preserve source order

Spreadsheet format codes can interleave literals, value-dependent fields, and
layout directives. Their relative order is meaningful and must survive every
stage of the pipeline.

The compiled `Operation` sequence and semantic output therefore retain source
order rather than assembling independent fragments that are repositioned later.
This is especially important for signs, prefixes, suffixes, fill, and spacing.

### Separate semantic formatting from layout

Value semantics and layout are different concerns:

- semantic formatting decides which digits, date components, text, symbols,
  and literals should appear;
- layout decides how deferred fill and spacing directives become output.

Semantic evaluators emit ordered `RenderPart` values. A single final layout
stage turns those parts into a string. Layout policy must not leak into numeric,
date/time, fraction, scientific, or text evaluation.

`fill_count` is a runtime layout option. It is an explicit repetition count,
not a measured cell or font width. Keeping it at the final boundary allows the
same prepared format to be reused with different presentation requirements.

### Keep formatting domains focused

Standard numbers, scientific notation, fractions, dates and times, text, and
large integers have distinct semantic rules. Each domain should own its
value-dependent behavior while sharing section selection, ordered output, and
layout resolution.

Shared mechanisms should capture genuine invariants, not force different
domains into one generalized algorithm.

### Keep runtime policy out of prepared formats

`FormatOptions` carries policy such as locale, date system, and fill count for
an individual formatting call. These options must not become hidden mutable
state or part of `NumberFormat` or its compiled plans.

Consequently:

- changing runtime options does not require reparsing;
- cache keys depend on format identity, not presentation options;
- concurrent callers cannot affect one another through a shared format.

### Treat compatibility as a boundary

Section selection, conditions, sign behavior, special numeric values, text
sections, and fallback behavior are observable spreadsheet semantics. Internal
refactoring must preserve them unless a deliberate compatibility change is
specified and tested.

Fallible APIs returning `FormatError` are the canonical execution paths.
Convenience APIs may provide compatibility fallbacks, but those fallbacks must
be deterministic and must not panic.

## Core invariants

The following invariants should hold across implementations:

- A `NumberFormat` contains at most four sections.
- Its `Section` values and compiled section plans describe the same sections in
  the same order.
- `CompiledFormat` contains only facts derived from the format code.
- Per-call values and `FormatOptions` remain local to that call.
- Semantic output preserves the relative order of the source format.
- Fill and spacing directives remain deferred in `RenderPart` values and are
  resolved only by the layout stage.
- Output-size arithmetic is checked, and excessive output is reported as an
  error rather than causing overflow or a panic.
- Formatting does not mutate `NumberFormat` or its compiled plans.

## Extending the formatter

Place a change at the earliest stage that can own it completely:

1. Recognize and validate new syntax during parsing.
2. Normalize syntax when the rule is independent of the formatted value.
3. Prepare reusable plan information when it depends only on the format code.
4. Evaluate behavior in the relevant formatting domain when it depends on the
   input value or runtime options.
5. Defer fill and spacing behavior to layout resolution.

When extending the implementation:

- avoid exposing internal execution structures through the public API;
- avoid rescanning syntax during every formatting call;
- avoid producing final strings inside domain logic when ordered intermediate
  output is required;
- avoid adding mutable evaluation state to reusable formats;
- prefer a focused domain abstraction over conditionals spread across the
  parser, evaluator, and layout stages.

## Testing principles

Tests should protect behavior at architectural boundaries:

- parsing tests cover recognition, ambiguity, validation, and normalization;
- formatting tests cover each semantic domain and section-selection behavior;
- layout tests cover ordering, Unicode, empty layout, and excessive output;
- public API tests cover errors, fallbacks, runtime options, and compatibility;
- cache tests verify that values and runtime options do not leak between calls.

Regression tests should describe observable behavior rather than private data
structures. Tests of internal preparation are appropriate only for invariants
that cannot be verified reliably through the public API.

## Non-goals

- The crate does not measure fonts, rendered glyphs, or spreadsheet cell
  geometry.
- The architecture names important private boundary types, but does not
  prescribe their fields, variants, helper types, or module locations.
- Internal execution data is not a public compatibility surface.
- This document is not a complete specification of OOXML number-format syntax;
  detailed behavior belongs in user-facing documentation and tests.

## When to update this document

Update this document when a pipeline boundary, ownership rule, public
compatibility policy, or core invariant changes. Do not update it for routine
refactoring, renaming, moving modules, changing private data structures, or
replacing an algorithm within an existing boundary.
