# Kujo 1.10.0 candidate release notes

Kujo 1.10.0 is an unpublished candidate. [Kujo 1.9.0](RELEASE_NOTES_1_9_0.md)
is the current stable release.

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
macOS and Linux paths. Released ownership guards preserve returned values,
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

The candidate updates the locked capability, SQLite, compression, TLS,
cryptographic and I/O dependencies for applicable RustSec advisories.
Date and timezone handling now uses maintained dependencies and a bundled IANA
database for cross-platform consistency. The artifact workflow requires a
strict audit before builds.

## Compatibility and upgrade notes

The v1 language and machine-readable compatibility policies still apply.
Consumers that relied on a non-callable value yielding zero in the interpreter
must handle the runtime error. The proposed native and npm cohort covers Linux
x64 and arm64, macOS x64 and arm64, and Windows x64. The VS Code extension and
crates.io distribution have separate publication requirements.

## Known limitations

The pinned Windows JIT backend can retain executable memory after a VM is
destroyed. The public standalone compiler lifetime contract is unchanged.
The deferred worker reuse, scalar and field experiments require further
acceptance. LangArena production completion and structural equivalence across
all 50 workloads remain open. Wave C beta, Wave D alpha and unpublished
participant SDKs retain their experimental scope.

## Verification evidence

See the [final readiness audit](audits/KUJO_V1_10_FINAL_RELEASE_READINESS.md)
for candidate test results, dependency findings and release blockers.
