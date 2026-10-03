# Kujo 1.8 release-readiness record

Date: 2026-10-03

Starting source: `656c03c3c916e5fd25508438b9a6808e88263ba9`

This tranche closes feature development for Kujo 1.8 and moves the branch to
release-candidate preparation. Publication is not authorized or performed here.

## Completed work

- **Compiler/VM:** audited existing folding, dead-code, peephole and relocation
  passes. No speculative control-flow transform was added. Generator resume no
  longer clones a continuation-owned bytecode chunk.
- **Runtime:** re-measured calls, generators, closures, tasks, promises, channels,
  scheduling, rejection and cancellation. Only the generator-resume change was
  retained; the remaining deltas were noisy, inconsistent or too small.
- **Shared analysis/LSP:** imported-module dependencies are recorded in snapshots;
  open buffers override disk source; direct and transitive open dependents refresh
  after edits; close/deletion invalidates back to disk state.
- **Parity/optimizer safety:** fixed interpreter shadowing of preloaded native
  names while preserving duplicate-declaration errors. VM/interpreter, loop early
  return and generator continuation regressions cover the changed paths.
- **Representative workloads:** added deterministic offline data/CLI,
  multi-file import, generator-heavy, async/concurrent and mixed-application
  programs with both-engine output assertions.
- **Compatibility:** fourteen exact programs extracted from tag `v1.7.0` passed
  on the candidate in both VM and interpreter with identical output. Dispatch's
  complete offline release gate also passed against the candidate (101 source
  tests, 24 Dispatch shards and focused integration suites).
- **Release maintenance:** source and npm metadata are aligned at 1.8.0; candidate
  metadata, changelog, roadmap and release notes are synchronized. Kujo 1.7.0
  remains the latest published stable release.

## Performance disposition

The retained generator change improved the nested-alias workload by 6.7% and
6.8% at the paired median in two campaigns. A focused 20-run workload improved
20.0% by mean, with high variance explicitly recorded. No other runtime or
compiler optimization was retained. Full methods, raw samples, binary hashes and
variance notes are in
[`benchmarks/results/kujo-1.8-release-readiness-2026-10-03/`](../benchmarks/results/kujo-1.8-release-readiness-2026-10-03/README.md).

## Compatibility statement

- Syntax changes in this tranche: none.
- New migration burden: none for valid v1.7 programs.
- Intentional behavior correction: interpreter lexical declarations may shadow
  native function names, matching the VM.
- Diagnostic behavior: open-project LSP diagnostics now update when unsaved
  imported exports change; stale results are no longer retained.
- VM/interpreter differences introduced: none known.
- Capability, cancellation, async lifecycle, snapshot and effect ordering:
  unchanged.

## Release boundary

The branch is feature-frozen. Async generators, `yield from`, ownership cycles,
atomic shared state, generics, macros, broad FFI, distributed execution and a
major WASM target remain future roadmap items, not 1.8 blockers.

Final tagging, GitHub/native/npm publication, public installer promotion and the
five-target hosted artifact matrix remain owner-authorized release-execution
steps. Crates.io is not required for the canonical native/npm distribution.
