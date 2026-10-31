# Local struct field reads: verified allocation fix

Status: the scoped local receiver-copy fix and its regression gates pass. Broader
section 5 work remains open; this is not universal field-read or speed acceptance.

## Problem and implementation

The [baseline](STRUCT_FIELD_READ_BASELINE_2026_10_31.md) isolates the temporary
receiver copy in repeated local struct field reads. Growing the struct type name
from 8 to 256 bytes adds 248 requested bytes per VM read. JIT repeats show the
same scale, with compiler-allocation variability. Extracting the field before
the loop removes that scaling in the control.

The compiler now emits `FieldGetLocal` for a field access whose receiver resolves
to a local slot. The VM borrows that receiver and the opcode's field metadata.
Eligible native locals lend their existing root to the canonical field helper;
the returned value still receives independent ownership. The input root is not
released by the read. Existing lookup, errors and method-receiver handling remain
in the canonical helper.

Native loops retain callback inspection and reconstruct the VM stack without an
extra receiver when resuming at this opcode. Specialized integer-dictionary loop
admission retains the restriction previously imposed by `LoadLocal`.

This does not remove every field-read cost. Interpreter execution is unchanged;
non-register native locals retain their existing lookup fallback. Captured/global
receivers, other receiver expressions and explicit method-call lowering are not
claimed optimized by this change. Returned nested values still require their own
value-semantic ownership.

## Verified targeted evidence

Candidate executable SHA-256:
`30ebacb7fbc09ae25640c849b5517dac0a034855a9234310fffa6fbdea34a49d`.
Candidate library SHA-256:
`8becd18b137d9267eddd620e6ecb69f2b01a5e449fe3cffce8c6614df64a09f5`.

- 14 targeted tests pass; one existing manual depth-20 resource test is ignored.
  Portable contracts run in VM, interpreter and JIT. The dictionary-dot regression
  specifically covers existing VM/JIT behavior; it does not extend the interpreter
  contract beyond portable bracket indexing.
- 128 allocation observations and 192 semantic executions pass. Exact fixture
  membership, raw counters and artifact identities are checked.
- All 64 native-entry cases pass; 48 additional JIT allocation observations retain
  compiler variability rather than treating each run as an exact constant.
- All 64 VM allocation pairs are non-increasing: 32 lower and 32 unchanged. The
  isolated type-name slope is zero in every target/control combination.
- Initial JIT pairs contain 51 reductions and 13 increases. All increases occur
  in dictionary controls. Four paired repeats per increased case produce 104
  additional observations; baseline/candidate ranges overlap in all 13 cases.
  The raw increases remain part of the evidence, not discarded outliers.

Repeated JIT inline type-name slopes are -4.079 to -0.092 bytes/read for scalar
fields and -0.260 to -0.113 for row fields. Extracted controls range from -0.013
to +4.909 and -0.056 to +0.224 respectively. These differential observations
support removal of the approximately 248-byte effect; negative slopes are not
negative allocation, and this is not an exact zero-cost JIT claim.

## Rejected candidate and corrective regression

The first candidate passed the original targeted tests but failed a mixed
integer-dictionary/field-read loop with `Undefined variable: data` after native
entry. `FieldGetLocal` had bypassed a specialized-loop restriction formerly
triggered by `LoadLocal`. The corrected admission check preserves that restriction.

The permanent regression passes on the accepted baseline and fails on the first
candidate with that exact error. It passes on the corrected candidate with native
loop execution. The failed candidate, output and negative-control result are
retained under `.audit-evidence/local-field-fix-2026-10-31` and the V2 audit.

## Priority workloads and final gates

All 150 test-size checksum checks pass: 50 each in VM, interpreter and JIT.
The library suite passes 1,309 tests, with eight existing manual tests ignored.
The selected integration suite passes 433 tests, with one existing manual test
ignored. Exact test membership is checked against each executable's `--list` output.
All 16 priority allocation pairs and eight matching interpreter checks pass their
checksums. These are execution allocations, including setup and profiling output:

| Workload | VM requested-byte change | JIT requested-byte change |
| --- | ---: | ---: |
| QuickSort | +14 | +5,042 |
| MergeSort | -31 | +15,576 |
| NeuralNet | -13,323,122 (-1.018%) | -7,088,376 (-0.703%) |
| CacheSimulation | +186,112 | +35,564 |
| GameOfLife | -2,128 | -320,954 |
| TextRaytracer | -19,503,147 (-2.458%) | -18,345,360 (-3.951%) |
| Brainfuck Array | -30 | +19,304 |
| Brainfuck Recursion | 0 | -14,248 |

All six increased pairs receive three additional paired rounds (36 executions).
Both request-count and byte ranges overlap for each increased pair. In a separate
QuickSort diagnostic using the existing interface without optional profiling
output, two paired rounds produce identical counts in both builds: 11,410,012
requests and 476,797,099 bytes. Its algorithm, configuration and checksum are
unchanged. This isolates profiling-related variation without altering the actual
benchmark port or substituting those diagnostic results for the profiled results.

The deterministic field-read VM matrix retains a strict no-increase requirement.
The whole-workload priority review retains initial increases and requires paired
evidence rather than assuming exact allocation determinism. Overlapping ranges
do not prove zero regression; no improvement is inferred for those pairs.

## Limits and provenance

The V2 executable initially failed to link because the disk was full. Regenerable
debug cache was cleared; the unchanged release build then passed. Its failed log
is retained. Independent filesystem clones preserve artifacts without redundant
physical copies. This storage incident is not a runtime test failure.

Requested allocation traffic is not elapsed speed or retained/peak memory. This
report makes no Goose/Rust competitiveness, production-size, platform or clean-host
claim. No benchmark algorithm, language contract or maintainer interaction changed.
Detailed local evidence is in `.audit-evidence/local-field-fix-v2-2026-10-31`.
