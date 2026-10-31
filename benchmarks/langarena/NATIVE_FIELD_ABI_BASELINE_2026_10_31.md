# Native field-read ABI baseline

Accepted runtime `26105f4` still pays a measurable direct-helper cost for reading
ordinary struct fields. The diagnostic links against its saved library
`8becd18b137d9267eddd620e6ecb69f2b01a5e449fe3cffce8c6614df64a09f5`.
Six rounds alternate field/array order and reverse payload order; each measured
case performs 200,000 reads after 100 warmup reads. All 60 observations pass scalar
kind/checksum checks. Warmup checks exact scalar payloads (including negative zero)
and nested-array contents; owned results are independently rooted and released.

| Payload | Array control ns/read range | Struct field ns/read range |
| --- | ---: | ---: |
| Integer | 11.62–12.44 | 127.68–132.76 |
| Negative-zero float | 11.53–11.78 | 120.91–130.02 |
| Boolean | 11.67–13.73 | 115.74–129.57 |
| Null | 10.75–11.92 | 115.96–127.58 |
| Nested array | 90.81–94.20 | 195.35–204.91 |

These are local diagnostic ranges, not stable speed estimates or evidence that
all of the difference is copying. Struct lookup also hashes a field name; array
lookup indexes directly. The [current profiles](CURRENT_KERNEL_ATTRIBUTION_2026_10_31.md)
independently attribute field-read, root-encoding and memory-movement work.
A candidate must retain the array control and owned-result case, preserve method
fallbacks, and demonstrate its own improvement before acceptance.

`tests/jit_field_scalar_abi.rs` passes on the unchanged baseline in VM, interpreter
and JIT, asserting actual native entries in JIT. It covers mixed scalar tags,
owned-result isolation, a non-struct error and recovery, and legacy parser method
precedence. This is a semantic baseline, not verification of the candidate patch.

## Existing parity difference

A user-declared `ArgParser` with a stored integer `help` field returns a method
marker from `parser.help` in VM/JIT, but the interpreter returns the integer.
The regression explicitly preserves those existing results during optimization.
Choosing a uniform language behavior is a separate unresolved parity decision;
this performance work does not silently change it.

## Reproduction and evidence

Build `probes/native_field_read_cost.rs` with `rustc --edition=2021 -O -C lto=thin`,
`--extern kujo=PATH/libkujo.rlib` and `-L dependency=target/release/deps`.
Run the resulting executable and retain all 60 output lines. Before/after builds
must use the same probe source. The direct helper does not execute generated code
and does not establish end-to-end native CPU coverage. Timing includes validation
branches and black-box calls, excludes root setup and warmup, and remains sensitive
to laptop activity. No clean-host or competitive performance claim follows.

[Evidence](native-field-abi-baseline-2026-10-31.json.gz) preserves sources, raw
observations, binary/library hashes and the regression receipt. The initial probe
failed to compile because it accessed a private context field; the corrected probe
checks the observable receiver stack instead. No runtime change was needed for
this baseline.
