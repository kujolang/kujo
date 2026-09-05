# Kujo Field Notes — v1.3.0 Release Readiness

**Date:** 2026-09-05
**Session:** 16:43 local
**Branch/Commit:** codex/release-v1.3.0 / a747a0ad609fbcc31f5174641604bba0e3b2664b
**Scope:** Signed GitHub binary release readiness and current-product documentation

---

## What I Changed

Prepared Kujo v1.3.0 as an additive minor release from committed runtime source
0d692c1eedf0cd05768a7cf4ac42e88139fd2861. The release covers native runtime
upgrades, streaming HTTP/file/network APIs, PostgreSQL TLS and async execution,
SQLite inspection/transactions, strict charset decoding, and VM/async fixes.

Aligned Cargo and npm metadata, refreshed generated TODO/unsafe inventories,
made the npm resolver version assertion follow package metadata, and updated
runtime-upgrade examples to the first supporting release. Reviewed the one
additional executable unsafe occurrence: a test-only, no-argument libc::geteuid
call used to skip a permission assertion when running as root. The exact unsafe
budget is now 62; no production unsafe boundary was added by release preparation.

The user explicitly requested a new release, signed tag, and GitHub publication
in this release-execution context. That request authorizes this release and
supersedes the earlier no-release instruction from feature implementation.
Cargo registry publication is excluded; GitHub binaries remain canonical.

## Gotchas (Read This Next Time)

- Isolate release work when another task is editing the shared checkout. The
  original checkout's unrelated crypto/documentation edits were preserved and
  were not included in this release candidate.
- The local host can exhaust process resources (EAGAIN/OS error 35), as already
  documented in v1.2.3 release readiness. Require the clean hosted full gate.
- A custom Cargo target directory needs a compatibility runner at the repository's
  target/debug/kujo path for the VM inventory freshness test. The isolated
  checkout used a local ignored symlink to the exact compiled candidate binary.
- Do not publish a normalized Cargo crate that loses the repository-local
  hardened tiny_http patch. This release does not attempt crates.io publication.
- Public stable-release claims stay on v1.2.3 until v1.3.0 artifacts and published
  smoke checks succeed. Website upgrade instructions must then say available
  from v1.3.0; product-site copy stays general rather than narrating the feature.

## Things I Learned

- Generated inventory drift and a stale npm-version expectation were release
  preparation defects, not reasons to relax runtime behavior or skip checks.
- Strict upgrade ownership, runtime-only scope, and Windows backup recovery
  remain relevant operational documentation after release.

## Debug Notes (Only if applicable)

| Command / evidence | Result |
| --- | --- |
| cargo fmt --check | PASS locally and in CI. |
| cargo clippy --all-targets --all-features -- -D warnings | PASS; existing vendored tiny_http warnings remain outside first-party diagnostics. |
| bash scripts/release_gate.sh --full | PASS in clean CI run 33989660983 for a747a0ad609fbcc31f5174641604bba0e3b2664b, including all prerequisite jobs and the full release gate. |
| Native upgrade / no-default-features matrix | PASS; run 33989660988 covers Linux x64/arm64, macOS x64/arm64, and Windows x64. |
| Filesystem capability conformance | PASS, run 33989660949. |
| LSP matrix | PASS, run 33989660963. |
| Artifact validation matrix | PASS, run 33989660948. |
| Release-state / contract metadata guard | PASS, run 33989660984. |
| npm test --prefix npm | PASS, 12 tests. |
| npm run pack:dry-run --prefix npm | PASS, version-aligned runtime/platform packages. |
| ShipCheck scan and gate against kujo-release-v1.3.0 | Exit 0; 12/16 checks pass, zero errors, four documented Cargo-detector warnings under docs/SHIPCHECK_RELEASE_EXCEPTIONS.md. |
| Local full-gate attempts | HOST-CONSTRAINED: timing fixtures passed independently; a later attempt reached integration tests before process creation failed with EAGAIN. No full-local-pass claim is made. |

CI is the complete release-gate evidence. Local command logs remain under
/tmp/kujo-v130-*.log. Release preparation changed metadata, documentation,
generated inventories and version/budget assertions; runtime implementation
remains frozen at the committed source revision named above.

## Follow-ups / TODO (For Future Agents)

- Publish only after the signed tag's release workflow passes its full gate and
  all five native artifact builds. Verify the tag signature through GitHub.
- Run published-artifact smoke, then promote current stable-release references
  and deploy the staged docs content. Do not treat a Cargo version bump as publication.
- Keep package-registry outcomes separate from GitHub binary availability.

## Links / References

- Full gate: https://github.com/kujolang/kujo/actions/runs/33989660983
- Native upgrade matrix: https://github.com/kujolang/kujo/actions/runs/33989660988
- Filesystem matrix: https://github.com/kujolang/kujo/actions/runs/33989660949
- LSP matrix: https://github.com/kujolang/kujo/actions/runs/33989660963
- Artifact matrix: https://github.com/kujolang/kujo/actions/runs/33989660948

Release-readiness sign-off: the clean hosted full gate and all required matrices
passed. The candidate is ready for the signed v1.3.0 tag. Tagged binary
publication and post-publication smoke remain separate required gates.
