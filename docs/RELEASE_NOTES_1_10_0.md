# Kujo 1.10.0 release notes

Published on 2026-11-20 from signed source `5d1eeca402f0dfbd1450452f796417fad0c5aa00`.
Native archives and npm packages cover all five supported targets. See the
[publication record](../release/kujo-1.10.0-publication.json).

## Major runtime changes

The VM reuses eligible compiled loops across calls. Admission is scoped to the
chunk and physical local slots. Runtime guards still validate collection shapes
on entry, and repeated native fallback is subject to bounded backoff.

## Language correctness

Invalid call arity and calls to non-callable values now reach VM exception
handlers. The interpreter reports non-callable values as errors. Checked nested
writes preserve snapshots and evaluation order, including prior writes when a
later operation fails. JSON parsing correctly rounds decimal and exponent
tokens around the IEEE-754 safe integer boundary.

## Performance improvements

The VM reduces copying on eligible captured array, dictionary and struct
updates. Reusable local and capture metadata, cached depth checks and selected
named and field reads reduce repeated work. The
[performance consolidation](../benchmarks/langarena/PERFORMANCE_CONSOLIDATION_2026_11_04.md)
records repeatable Quick JIT computation ratios of 1.056× and 1.063× in two
historical comparison sessions. Merge timing was inconclusive. Measurements
apply to the pinned workloads and builds.

## Memory and JIT improvements

VM-private executable memory is reclaimed after VM destruction on verified
macOS and Linux paths. The ownership changes preserve returned values,
callbacks and suspended generators. Eligible native SSA local stores transfer
owned values while preserving initialization, error and snapshot behavior.

## Standard library and native APIs

Use the [changelog](../CHANGELOG.md) for individual native API corrections and
the [standard library reference](STANDARD_LIBRARY.md) for signatures, bounds
and capability requirements.

## Developer tooling

JIT statistics account for cached loop sites and compiled variants. The separate
VS Code extension remains at version 0.1.0. Its guarded language-client
adaptation retains file-operation matching with the updated minimatch API.

## Security and dependency updates

This release updates the locked capability, SQLite, compression, TLS,
cryptographic and I/O dependencies for applicable RustSec advisories.
Date and timezone handling now uses maintained dependencies and a bundled IANA
database for cross-platform consistency. The artifact workflow requires a
strict audit before builds.

## Compatibility and upgrade notes

The v1 language and machine-readable compatibility policies still apply.
Consumers that relied on a non-callable value yielding zero in the interpreter
must handle the runtime error. The native and npm release covers Linux
x64 and arm64, macOS x64 and arm64, and Windows x64. The VS Code extension and
crates.io distribution have separate publication requirements.

## Known limitations

The pinned Windows JIT backend can retain executable memory after a VM is
destroyed. The public standalone compiler lifetime contract is unchanged.
The course parity probe still selects the default branch for qualified custom-enum
matches in the VM and matches the declared variant in the interpreter.
The deferred worker reuse, scalar and field experiments require further
acceptance. LangArena production completion and structural equivalence across
all 50 workloads remain open. Wave C beta, Wave D alpha and unpublished
participant SDKs retain their experimental scope.

## Verification evidence

- [Exact-source CI gate](https://github.com/kujolang/kujo/actions/runs/44733010328)
- [Locked Rust suite](https://github.com/kujolang/kujo/actions/runs/44739923350)
- [Five-platform rehearsal](https://github.com/kujolang/kujo/actions/runs/44733061322)
- [Tagged publication](https://github.com/kujolang/kujo/actions/runs/44744812698)
- [Published native install and upgrade](https://github.com/kujolang/kujo/actions/runs/44756212895)
- [Published npm installation](https://github.com/kujolang/kujo/actions/runs/44756712657)

The legacy benchmark directory reported the same 50 failed fixture/runtime pairs
in the published v1.9.0 baseline and the candidate rehearsal. The legacy runner
exits successfully even when fixture results fail. Performance acceptance comes
from the pinned workload receipts.
