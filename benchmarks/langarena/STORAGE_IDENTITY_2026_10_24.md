# Allocation-free root storage identity — 2026-10-24

Runtime `9f401eb`, compared with `93e7caa`. The shared `same_storage` helper now
compares array, byte and dictionary root pointers directly. It allocates a
traversal vector only for structs. Scalar rejection, mismatched types, nested
struct traversal, field-name checks and floating-point bit comparisons remain
unchanged. No new kind of value is eligible for ownership release.

The earlier extracted-helper prototype is now integrated and tested. Raw
allocation logs, source identities and regression results are preserved in
`storage-identity-runtime-2026-10-24.json`.

## Structural QuickSort comparison

Default-feature debug library on macOS x86_64, 8,192 elements:

| Configuration | Requests before → after | Requested bytes before → after |
| --- | ---: | ---: |
| VM, unique | 505,102 → 490,470 | 44,976,626 → 44,742,514 |
| VM, shared | 505,104 → 490,472 | 46,090,778 → 45,856,666 |
| JIT, unique | 58,534 → 43,902 | 9,278,520 → 9,044,408 |
| JIT, shared | 58,530 → 43,898 | 10,390,840 → 10,156,728 |

Each configuration removes 14,632 allocation requests and 234,112 cumulative
requested bytes. The unique JIT case uses 25% fewer requests and approximately
2.5% fewer requested bytes. These are allocation counts, not elapsed time or
peak-memory improvements.

Unique inputs retain their backing without array-sized allocations; shared
inputs detach once and preserve snapshots. Sizes 128, 1,024 and 8,192 pass
element-by-element output and ownership checks in both configurations. The
probe's new `--expect-storage-identity-budget` gate requires fewer than 45,000
requests in both 8,192-element JIT cases. It passes this candidate and fails
the previous runtime.

## Verification and remaining scope

- 1,092 library tests pass; 8 intentionally ignored.
- Two new unit tests cover aliases versus equal payloads, mismatched root types,
  unchanged scalar/string/integer-dictionary rejection, nested struct maps,
  struct names, signed zero, NaN payload bits and 128-level struct chains.
- 27 targeted integration tests pass, including VM/interpreter/JIT snapshots,
  callback rebinding, call-frame ownership, helpers and loop recovery.
- No-default-feature compilation, formatting and diff checks pass.

The frozen-release checksum/timing job currently uses the preceding runtime,
`93e7caa`. Its 150 checksum cases passed; timing and memory samples are being
repeated after compilation overlap in the first pass. Those results do not
verify this newer runtime. This candidate still needs release verification;
production completion, argument-buffer costs, clean-server comparison and
supported-platform gates remain open.
