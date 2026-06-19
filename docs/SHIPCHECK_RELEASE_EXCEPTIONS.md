# ShipCheck Release Exceptions For Kujo Runtime

Date: 2026-06-19
Status: active v1.0 release-readiness exception note

ShipCheck is useful as a broad repository release-readiness scan, but its current metadata detectors are optimized for Kujo package repositories and generic script projects. The Kujo language/runtime repository is a Rust crate with Cargo-owned release metadata, so the warnings below are intentional unless ShipCheck adds first-class Rust/Cargo metadata detection.

## Format Command Warning

ShipCheck warning: `No format command detected`.

Release decision: intentional exception.

Canonical command:

```bash
cargo fmt --check
```

Reason: formatting is already enforced by `scripts/release_gate.sh --full` and `scripts/release_candidate_gate.sh --full`. Adding a Makefile or `kennel.toml` solely for detector discovery would duplicate the canonical release gate command source.

## Lint Command Warning

ShipCheck warning: `No lint command detected`.

Release decision: intentional exception.

Canonical command:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Reason: linting is already enforced by `scripts/release_gate.sh --full` and `scripts/release_candidate_gate.sh --full`. The release gate remains the source of truth for exact flags.

## Kennel Manifest Warning

ShipCheck warning: `No kennel.toml found`.

Release decision: intentional exception.

Reason: this repository publishes the Kujo language/runtime crate and binary through Cargo and GitHub release artifacts. It is not itself a Kennel package repository. Kennel registry/package launch boundaries remain tracked separately as release-readiness follow-up work.

## Entry Point Warning

ShipCheck warning: `No clear entry point detected`.

Release decision: intentional exception.

Canonical entry point:

- Rust source: `src/main.rs`
- Binary name: `kujo`
- Package metadata: `Cargo.toml`

Reason: ShipCheck currently detects entry points from `kennel.toml` or root `.kujo` scripts. The Kujo runtime entry point is the Cargo binary, not a root Kujo script.

