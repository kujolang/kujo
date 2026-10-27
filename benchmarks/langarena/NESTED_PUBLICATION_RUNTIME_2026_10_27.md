# Nested scalar publication — 2026-10-27

Runtime commit `ad6868058c6f1dd899d304f8edd35011bf59cafa` removes per-call row-width copying for eligible
imported helpers that repeatedly update one scalar element of a struct-owned or
dictionary-owned array. The private write reuses the already evaluated assignment
path. Public values remain unchanged before publication, and retained snapshots
still detach through ordinary copy-on-write behavior.

Baseline runtime: `868dfd8`. Exact source, executable, library, probe, compiler
arguments and raw results are in [the machine evidence](nested-publication-runtime-2026-10-27.json).
These are local allocation and correctness results, not elapsed-speed claims.

## Allocation results

Added 32 helper calls, one write per call, retaining the original snapshot once:

| Runtime | Owner | Row width | Baseline requested bytes | Candidate requested bytes | Candidate requests |
| --- | --- | ---: | ---: | ---: | ---: |
| vm | struct_array | 128 | 885344 | 303680 | 3008 |
| vm | struct_array | 2048 | 9241184 | 303680 | 3008 |
| vm | dict_array | 128 | 887232 | 305824 | 2816 |
| vm | dict_array | 2048 | 9243072 | 305824 | 2816 |
| jit | struct_array | 128 | 885344 | 303680 | 3008 |
| jit | struct_array | 2048 | 9241184 | 303680 | 3008 |
| jit | dict_array | 128 | 887232 | 305824 | 2816 |
| jit | dict_array | 2048 | 9243072 | 305824 | 2816 |

Both owner shapes also have flat added-call allocation across these widths with
eight writes per helper. VM and JIT agree. In the complete module matrix,
32 of 128 paired cases allocate fewer bytes, with no request/byte
regressions. Fresh snapshots remain subject to required copying; this does not
claim that all nested updates are constant-cost.

## Ownership and fallback

Admission is limited to an eligible synchronous private capture, a mutable
unshadowed root, the expected root/row ownership counts, a valid two-level path
and a scalar value. Repeated writes coalesce only at the same row element.
Other paths and values use the existing validated update behavior.

Pure identifier-rooted field reads preserve unused admission. Pending writes
still materialize before exposing field values or crossing unknown operations.
A nested write yields to an eligible direct-array or direct-dictionary write on
another binding, using the existing admission rules. If direct admission fails,
the original nested journal remains intact.

Regression coverage includes retained and escaped snapshots, public-owner reads,
callbacks, shadowing, immutable bindings, scalar types, canonical integer keys,
invalid indices, completed writes before errors, and replacement-array ordering.

## Verification

- 1,177 library tests pass; eight existing ignored tests remain ignored.
- Eight nested-assignment tests and three expanded VM/interpreter/JIT contracts
  pass on the final source. The expanded contracts also pass the unchanged baseline.
- All 339 matching release integration tests pass; one manual profile is ignored.
- All 18 baseline/candidate direct nested-assignment parity runs pass.
- All 128 module allocation pairs and 192 semantic runs pass.
- All 72 additional allocation pairs and 108 semantic runs pass, covering rows
  of 2/128/2,048 elements, dense/other-row/escape behavior and competing direct
  array and dictionary writes, with no request/byte regressions.
- All 16 read/write-key pairs, 12 numeric-key pairs, 36 cold comparisons, eight
  append pairs and three publication-boundary controls pass.
- All 150 test-size benchmark checksums pass: 50 each in VM, interpreter and JIT.
- Format, diff, frozen source, benchmark source and artifact identity gates pass.

The first candidate passed correctness but failed acceptance: struct rows still
copied with width, and pending nested writes blocked existing direct collection
optimizations. Expanded controls exposed ten allocation regressions in that
candidate. The evidence retains both rejected measurements and the accepted
revision; the acceptance requirements were not weakened.

## Remaining scope

General captured-binding traversal/name copying, oversized-owner fallback,
broader nested paths, dense or escaping update costs and wider dictionary cases
remain open in the [full checklist](PERFORMANCE_CHECKLIST.md). This checkpoint
does not establish peak-memory, production-size, platform or clean-host
Goose/Rust acceptance. No Kostya contact occurred.
