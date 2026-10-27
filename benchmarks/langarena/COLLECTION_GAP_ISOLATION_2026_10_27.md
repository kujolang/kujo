# Remaining collection costs on `82bdef6` — 2026-10-27

The accepted runtime still copies collection-width storage per imported helper
call for multiple dictionary keys, floating-point dictionary updates and several
nested shapes. This is a reproduction of remaining costs, not an optimization
or elapsed-speed result. No runtime code changed in this characterization.

[Machine evidence](collection-gap-isolation-2026-10-27.json) records 240 allocation
measurements and 360 successful VM/interpreter/JIT semantic checks. Inputs cover
10 shapes, widths 2 / 128 / 2,048, 16 / 32 helper calls and two snapshot lifetimes.
VM and JIT allocation deltas agree for every group. Artifact hashes match the
accepted [compact-capture release evidence](compact-capture-views-2026-10-27.json).

## Added-call cost

The table subtracts the 16-call total from the 32-call total and divides by 16.
An initial snapshot remains retained, but subsequent helper calls do not create
fresh snapshots. Construction aliases are explicitly cleared before measurement
calls so they do not accidentally keep the current collection shared.

| Update shape | Bytes/call at width 128 | Bytes/call at width 2,048 |
|---|---:|---:|
| One integer dictionary key | 4,382 | 4,382 |
| Two integer dictionary keys | 43,657 | 631,177 |
| One floating-point dictionary key | 43,657 | 631,177 |
| One element of a struct-owned array | 4,802 | 4,802 |
| Two elements of a struct-owned array | 23,316 | 284,436 |
| Two elements of a dictionary-owned array | 23,350 | 284,470 |
| One element of an array-owned array | 22,342 | 283,462 |
| One element through a three-level dictionary path | 23,918 | 285,038 |
| One key of a dictionary-owned dictionary | 44,755 | 632,275 |
| One key of a struct-owned dictionary | 44,760 | 632,280 |

Request counts remain constant with width in these groups; requested bytes grow.
The already optimized one-key integer dictionary and one-element struct-array
routes provide flat controls. Existing nested publication coalesces only the same
row element; dictionary publication accepts one existing key with an integer value.
The other shapes take ordinary mutation paths.

The separate fresh-snapshot groups retain the current value before every helper
call and verify that each snapshot remains unchanged afterward. Their width costs
must not be removed by dropping ownership or changing value semantics. An improved
representation could change copying costs, but this run does not establish one.

## Next integration candidate

Investigate several scalar updates to the same nested row using the assignment
path storage already allocated by ordinary nested assignment. A component layout
probe on this macOS x64 build reports:

- `Value`: 136 bytes, alignment 8.
- Existing row-write model: 48 bytes, alignment 8.
- Optional row entry: 24 bytes, alignment 8.
- Four-entry row-write model: 120 bytes, alignment 8.

This is a necessary storage-fit check, not proof that actual `Access`, capture
journals or frames remain unchanged. Integration must verify their actual layout,
all scalar values, negative indices, repeated/different indices, bounded overflow,
snapshots, errors, public-owner visibility, competing writes and all runtimes.
Use the existing repeated-one-element path and fresh-snapshot cases as controls.
Reject regressions rather than relaxing those controls after measurement.

Multiple dictionary keys/values, other nested roots and deeper paths remain open
alongside this candidate; this step does not close the broader collection item.

## Reproduction

Run [collection-gap-isolation.rb](probes/collection-gap-isolation.rb) with an accepted
allocation probe, its matching release executable and a new output directory.
The runner excludes elapsed timing from its decisions and preserves exact fixtures.
[The layout probe](probes/nested-row-slot-layout.rs), matching library hash and
compiler arguments are retained in the evidence. It does not modify the runtime.
No production-size, competitive-speed or platform acceptance is claimed.
