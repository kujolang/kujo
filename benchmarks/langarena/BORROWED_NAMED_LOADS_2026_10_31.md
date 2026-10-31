# Borrowed named-load opcode metadata

Runtime `a1f1858` removes the reproduced main-VM opcode-name copies. The
[baseline](NAMED_READ_BASELINE_2026_10_31.md) measured 248 additional allocated
bytes per global read when an identifier grows from 8 to 256 characters. The
candidate's length-by-iteration penalty is **zero** in both call-count controls.
This is an allocation result, not an elapsed-time or competitive speed claim.

The VM resolves `LoadVar` and `LoadGlobal` while borrowing immutable opcode
metadata, before the general opcode clone. Lookup order, lazy builtins, value
cloning, diagnostic strings and dynamic debug-environment checks are preserved.
The instruction pointer advances at the same point. No reference survives the
operation; the inline callback dispatcher and native ownership representation
are unchanged. The moved handler bodies and exact string literals were compared
against the parent source, in addition to runtime testing.

## Measured effect

The independent verifier passes 32 allocation pairs and 48 semantic cases,
including local-read controls, lengths 8/256, loops 64/1,024 and calls 1/8.
No VM control increases requested bytes or allocation requests. The verifier
rejects the unchanged baseline for retaining the name-length penalty.

For eight calls of 1,024 global reads with a 256-character name, requested bytes
fall from 2,181,575 to 84,273 and allocation requests from 24,774 to 16,548.
With the 8-character name, bytes fall from 146,983 to 81,297. Startup and
per-call name costs remain; this does not eliminate every name-related cost.

All eight unchanged priority workloads request fewer allocation bytes in VM:

| Benchmark | Baseline bytes | Candidate bytes | Change |
| --- | ---: | ---: | ---: |
| Sort::Quick | 488,862,602 | 477,452,187 | -2.334% |
| Sort::Merge | 675,117,226 | 661,513,803 | -2.015% |
| Etc::NeuralNet | 1,317,311,052 | 1,308,167,466 | -0.694% |
| Etc::CacheSimulation | 514,679,661 | 514,453,467 | -0.044% |
| Etc::GameOfLife | 9,526,181 | 9,440,482 | -0.900% |
| Etc::TextRaytracer | 800,782,656 | 793,440,837 | -0.917% |
| Brainfuck::Array | 30,005,062 | 29,630,324 | -1.249% |
| Brainfuck::Recursion | 31,014,801 | 30,639,772 | -1.209% |

These process-wide execution counters include setup, warmup and checksums;
parsing and compilation of the initial source are excluded. They are neither
retained-memory measurements nor elapsed-time speedups.

The first JIT byte observations increased for QuickSort, NeuralNet, GameOfLife
and Brainfuck recursion. Three additional alternating-order runs per artifact
and case (24 executions) produced overlapping byte ranges in all four cases.
This does not establish a repeatable byte increase, but it also does not prove
zero regression or a JIT byte reduction. The evidence retains every observation,
including increases. JIT compiler allocation variability remains a limitation.

## Verification and identity

- The exact saved candidate passes 12 semantic cases across VM, interpreter and
  JIT, covering snapshots, closures, callbacks, builtins, long names and missing
  names with runtime-error exit code 4.
- All 150 test-size checksums pass: 50 each in VM, interpreter and JIT. Verification
  checks raw outputs, exact benchmark/config membership, executable identity,
  exit codes and truncation flags.
- Library tests: 1,309 passed, zero failed, eight existing manual performance
  tests ignored. Selected integration suite: 433 passed, zero failed, one
  existing manual byte-growth measurement ignored. Exact test membership is
  checked against each executable's test list; this is not every repository
  integration test.
- Sixteen priority allocation pairs, eight interpreter checks and 24 repeat
  executions pass independent identity, configuration, checksum and counter
  verification. Formatting and diff checks pass.

Candidate CLI SHA-256:
`112b272d0b378247d22a4814d551eaff0ec77c13b8e217f174fcae3c06bf3db2`.
The separately preserved initial test-build CLI has a different hash; acceptance
uses explicit regression tests bound to the saved measured candidate.
Source hashes bind both builds to the reviewed runtime source.

The [140-artifact evidence bundle](borrowed-named-loads-2026-10-31.json.gz)
contains the measurements, fixtures, raw logs, build/source identities, verifier
sources, negative control, exact test inventories and scoped acceptance record.
Every artifact digest is verified after the bundle is written. Intermediate
state files are historical evidence; `acceptance.json` and
`final-verification.json` define the final result.

## Remaining work

This closes the reproduced named-load opcode-copy cost. General capture/snapshot
costs and the broader section 3/4 requirements retain their previous dispositions.
Native ownership/indexed/field access and debug environment lookups remain
separate measured-profile candidates, requiring their own cost isolation before
changes. No new implementation is justified solely by a profile frame.

Production-size all-50 runs, platform acceptance, precise native/fallback CPU
coverage and clean-host Goose/Rust comparisons remain pending. No release,
upstream submission, benchmark algorithm change or Kostya contact occurred.
