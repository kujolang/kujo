# Kujo v1.1.0 Release Readiness Evidence

Date: 2026-08-29
Candidate commit: `7d1e60d7eb0cf57790e2ba0d333d272d4012c0f5`
Host: macOS 26.3.1 (25D771280a), Darwin x86_64
Toolchain: `rustc 1.96.0`, `cargo 1.96.0`, `cargo-audit 0.22.1`

## Scope

- CI-portable Agent project fixtures pinned to the scaffolded ecosystem commits.
- Reconciled and verified the Kujo-native repository gate example.
- Staged Cargo and changelog metadata for `1.1.0` while retaining `v1.0.2` as the published stable release.
- Hardened benchmark JIT preflight after the optional benchmark smoke exposed an unsupported-bytecode Cranelift panic.

## Verification

| Command | Result | Notes |
| --- | --- | --- |
| `cargo test --test repo_gate_example_contract` | PASS | 2/2 contracts passed. |
| `cargo test --test agent_project_contracts` | PASS | 11/11 contracts passed, including every Agent profile and live custom-provider bridge. |
| `cargo test 'benchmarks::'` | PASS | 97/97 benchmark module tests passed in each crate target. |
| `cargo run -- bench examples/benchmarks/sorting_algorithms.kujo --iterations 1 --warmup 0` | PASS | Unsupported JIT bytecode was reported as a bounded result; no panic; unavailable speedup rendered as `N/A`. |
| `KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_candidate_gate.sh --full` | PASS | Executed from a clean tree. Formatting, Clippy, all Rust tests, focused security/package/parity suites, socket tests, 145/145 Kujo fixtures, and `cargo audit --deny warnings` passed. `cargo-deny` was not installed and is optional under the canonical gate. |
| `bash .github/scripts/check-release-state.sh` | PASS | Source `1.1.0`, stable tag `v1.0.2`, and roadmap metadata aligned. |
| `cargo publish --dry-run --locked` | PASS | Packaged 1,015 files (6.9 MiB, 1.3 MiB compressed), rebuilt the packaged crate, and stopped before upload as required. |

## Sign-off

The `1.1.0` source candidate passes the canonical local release gates and crate publication dry run. Tagging and publication remain intentionally blocked until the maintainer supplies the required `UNBLOCK_V1_RELEASE` directive and completes the final release step.
