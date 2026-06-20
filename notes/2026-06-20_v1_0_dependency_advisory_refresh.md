# Kujo v1.0 Dependency Advisory Refresh

Date: 2026-06-20
Scope: v1.0 readiness dependency hygiene

## What changed

- Ran `cargo update` to refresh `Cargo.lock` to current SemVer-compatible crate
  releases.
- Verified `cargo check` after the lockfile refresh.
- Ran `cargo audit --ignore RUSTSEC-2023-0071`.

## Audit result

The lockfile refresh removed previously reported `core2` and
`proc-macro-error2` audit warnings. The remaining local audit output is warning
only:

- `RUSTSEC-2020-0168` (`mach`) through `cranelift-jit` for experimental
  optional JIT support.
- `RUSTSEC-2024-0436` (`paste`) through optional image lockfile metadata.
- `RUSTSEC-2023-0071` (`rsa`) remains explicitly ignored by
  `scripts/release_gate.sh` because the advisory has no fixed upgrade in the
  current dependency graph.

## Evidence

- `cargo audit --ignore RUSTSEC-2023-0071`: passed with warnings only.
- `cargo check`: passed.
- `tests/release_dependency_advisory_contract.rs` locks the removed-warning
  expectation and the remaining documented warning boundary.

## Follow-up boundary

No remaining dependency advisory item is locally actionable without either an
upstream crate migration or a broader optional-subsystem redesign. `cargo-deny`
remains deferred until that tool is installed and a repository policy can be
added deliberately.
