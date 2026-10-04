# Kujo 1.8 release-readiness record

Date: 2026-10-03

Starting source: `656c03c3c916e5fd25508438b9a6808e88263ba9`

This record began as the release-candidate readiness assessment. Kujo 1.8.0 was
subsequently published from the verified candidate on 2026-10-04.

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
- **Release maintenance:** source and npm metadata are aligned at 1.8.0;
  changelog, roadmap and release notes are synchronized. Kujo 1.8.0 is the
  latest published stable release.

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

## Final local validation

- `scripts/release_candidate_gate.sh --full`: passed, including formatting,
  Clippy, the complete Rust suite (977 library tests passed, 7 ignored, plus all
  integration suites), native security boundaries, package workflows, and 123
  VM/interpreter parity cases.
- Canonical Kujo fixtures: 150/150 passed in dual mode and 150/150 passed with
  the interpreter as primary; 11 provider or environment fixtures were skipped
  by their declared policy in each run.
- Socket-bound serve integration: 31/31 passed in the release-candidate wrapper.
- Supply chain: `cargo audit --deny warnings --ignore RUSTSEC-2025-0141` passed;
  `cargo-deny` was not installed locally and remains covered by hosted CI.
- Fuzz smoke: lexer, parser, bounded XML, bounded gzip, bounded single-entry ZIP,
  and PDF HTML-profile targets each ran for 10 seconds without a crash.
- The bounded loop-engineering verification passed formatting, locked checking,
  parity, optimizer, LSP reliability, and diff-integrity gates.

## Final packaging preparation

The 2026-10-04 preparation pass started from clean, synchronized `main` at
`06923afa141995fb6d4f2940a820515583676e16` and repaired the only failing
post-merge workflow: the root artifact-ignore contract now includes the
canonical `/var/` entry. Release automation now creates the source archive
declared by the 1.8 candidate metadata and the published-artifact smoke verifies
its checksum and embedded version contract.

The local macOS x64 rehearsal at
`59ca3e106baad2373cc336bf7cfa721474e165dd` produced a `kujo 1.8.0` binary and
both declared archives. The binary SHA-256 was
`99ff38f3d8c97f9fc89da76f72273c4eef14a01d41dcfbfc2eaccce327f82856`; the
native archive was
`bf4bbfcb71a053ae29fa2b2df457ffc68bbfdd409219cb7dcbf46644e178aa91`; and the
source archive was
`86bdaaf83596eb9212279a0e8e508b6cb1dcdc9daa4c9793fa68c0fc00b7ddab`.
The raw rehearsal receipt is retained at
[`docs/evidence/kujo-1.8-rc/local-rc-macos-x64.json`](evidence/kujo-1.8-rc/local-rc-macos-x64.json).

The full release-candidate gate passed again: 977 library tests passed with 7
ignored, all integration suites passed, 123 parity cases passed, both fixture
sweeps passed 150/150 with 11 declared skips, the 31-case serve suite passed,
and RustSec reported no unignored advisory or maintenance warning. npm tests
passed 12/12, all six 1.8.0 packages completed the dry-run pack, and the VS Code
extension static check passed. ShipCheck's generic gate passed 12/16 with four
documented non-blocking Cargo/runtime-repository detector warnings. `cargo-deny`
was not installed locally and remains a hosted-CI gate.

Hosted pull-request checks remain the authority for Linux, Windows, ARM64, and
the additional macOS configurations that this Intel macOS host cannot execute.

## Release boundary

The branch is feature-frozen. Async generators, `yield from`, ownership cycles,
atomic shared state, generics, macros, broad FFI, distributed execution and a
major WASM target remain future roadmap items, not 1.8 blockers.

The owner supplied the required release directive on 2026-10-04. The signed
`v1.8.0` tag, GitHub native/source assets, six npm packages, and both five-target
published-install matrices completed successfully. Crates.io was not required
for the canonical native/npm distribution. Final evidence is recorded in the
[publication note](../notes/2026-10-04_14-20_published-1-8-installation.md).
