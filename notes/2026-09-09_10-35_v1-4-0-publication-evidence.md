# Kujo v1.4.0 publication evidence

Date: 2026-09-09. Runtime release complete; Kennel publication remains separate.

## Published identity

- [GitHub Release v1.4.0](https://github.com/kujolang/kujo/releases/tag/v1.4.0), release ID 385585123, published 2026-09-09T14:22:33Z.
- Source commit: `266a8902068a14c3d17f803bef467dc28f1fe162`.
- Signed annotated tag object: `2c2b14ae3d6cddb325c3e5505cd5f43f498f48cd`; GitHub signature verification is valid. The tag and artifacts were not rewritten.
- Five native archives, five SHA-256 sidecars and consolidated checksums published.
- All six `@kujolang/kujo-*` npm packages published at 1.4.0, including the runtime wrapper and five platform packages.
- Main's subsequent documentation-only promotion updates published-stable references to 1.4.0. Contract schema versions remain unchanged.

## Verification

| Check | Evidence | Result |
| --- | --- | --- |
| Candidate full release gate | [34355522536](https://github.com/kujolang/kujo/actions/runs/34355522536) | Passed: format, Clippy, parity, minimal and full profiles |
| Tagged build and publication | [34357270998](https://github.com/kujolang/kujo/actions/runs/34357270998) | All nine jobs passed, including five native builds and GitHub/npm publication |
| Published artifacts and no-op upgrades | [34363189277](https://github.com/kujolang/kujo/actions/runs/34363189277) | All five platforms passed |
| Published upgrade from v1.3.1 | [34363309710](https://github.com/kujolang/kujo/actions/runs/34363309710) | All five platforms passed |
| Published npm installation | [34363305656](https://github.com/kujolang/kujo/actions/runs/34363305656) | All five platforms passed |

The candidate full-gate logs contain 79 passing Rust test summaries and 2,832
reported passing test executions, including repeated suites; these are not unique
test counts. The published smoke runs initially encountered a GitHub HTTP/2
transport error and a GitHub denied/rate-limit response. Failed jobs passed on
retry without changing source, tags, artifacts or assertions.

Local checks passed: `cargo fmt --check`, tag/version alignment, release-state
and contract-version guards, npm tests (12), npm pack dry run, and the five
affected documentation contract tests (`readme_contracts`,
`docs_policy_consistency_contract`, `architecture_docs_contract`,
`v1_maturity_boundary_alignment_contract`, `v1_scope_docs_alignment`).
ShipCheck scan/gate passed with zero errors and four existing detection warnings
for the Rust repository's lint/format/Kujo-entry/manifest conventions.

The actual published macOS x64 archive digest
`e0f41e86357d533f6a28a27c310a29deca21decb834f7baa550e894110e06ad5`
matched its sidecar, consolidated checksums and GitHub asset digest. Its binary
reported `kujo 1.4.0`. With that binary, Kennel's existing
`tests/native_global_tools_e2e.kujo`, `tests/native_global_security.kujo` and
`tests/native_package_smoke.kujo` passed. These validate native bootstrap,
idempotent profiles, global tool lifecycle, exact arguments/cwd, import isolation,
rollback/collisions, unsafe entries/permissions and deterministic USTAR round trips.
The lifecycle fixture deliberately traps Python invocation. No consumer Python
dependency was introduced. This verifies compatibility, not a Kennel release.

## Website evidence and boundaries

Documentation source: `kujolang/docs.kujolang.ai` commit
`0ce24cbc32939d378d6ead6ab064d55381c592c5`, verified build 34358820191,
published gh-pages commit `c210e39` (deployment 34364044927).
Main website source: `kujolang/kujolang.ai` commit `fb329ab`, verified branch
build 34357044934; production workflow 34364024731.
Both repositories retain their existing pinned runtime builds and hosting.
Their final production audit records live under
`seo-audit/release-1.4.0/2026-09-09/` in the corresponding repository.

The new installer/process operations are POSIX capabilities for Linux/macOS;
they do not establish native Windows Kennel support. The user-installed local
runtime and unrelated original checkout changes were preserved. Kennel's release
and published-runtime workflow pins are the next separately requested phase.

## Bootstrap follow-up

Production review found stale defaults in the canonical ecosystem installer
(v1.2.2) and its website copy (v1.1.0). Commit `7089ded` aligns the default with
v1.4.0 and adds a release-manifest test against Cargo's version. Website commit
`9c208a3` synchronizes the exact canonical script and its delivery contract.
An isolated real `--package dispatch` installation downloaded and verified the
published runtime, returned `kujo 1.4.0`, and passed the native upgrade check with
`status=up_to_date`, `changed=false`. Explicit pins and hostile-manifest rejection
tests passed. This changes the mutable installer on main; the released runtime
tag and native assets remain untouched.
