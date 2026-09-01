# Kujo Field Notes — v1.2.0 Release Readiness

**Date:** 2026-09-01
**Session:** 14:17 local
**Branch/Commit:** main / 3eea278ed1fbc89a142d99f3b5fe6a3bddf6b295
**Scope:** Kujo v1.2.0 runtime hardening, reusable CI setup, dependency audit, and clean release-candidate verification.

---

## What I Changed

- Prepared the additive runtime and CI work since v1.1.0 as the semver-minor v1.2.0 release.
- Refreshed generated release inventories before the clean candidate gate.
- Replaced yanked `mysql_async 0.37.0` with `0.37.1` after the dependency audit correctly blocked the first candidate run.
- Kept public stable-release claims at v1.1.0 until the v1.2.0 tag and artifacts are published and verified.

## Gotchas (Read This Next Time)

- The first clean candidate run was stopped by `cargo audit --deny warnings`
  because `mysql_async 0.37.0` had been yanked. Updating to `0.37.1` restored
  the warning-free dependency contract.
- Kujo fixture execution creates an untracked `tests/base64_utf8_test.out` file;
  clean-worktree automation must account for or remove that known test artifact.

## Things I Learned

- The additive network, TLS, compression, and setup-action surface requires a
  semver-minor release, so the correct successor to v1.1.0 is v1.2.0.
- A clean full gate must provide the pinned Agent ecosystem fixture root and
  explicitly enable socket tests to match tag-time coverage.

## Debug Notes (Only if applicable)

| Command | Result | Notes |
| --- | --- | --- |
| `KUJO_AGENT_FIXTURE_ECOSYSTEM_ROOT=/Users/robertdevore/2026/Kujolang/kujo-repos KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_candidate_gate.sh --full` | PASS | Clean detached worktree at `3eea278`; formatting, Clippy, all Rust tests, focused security/package/parity suites, 31 socket-bound serve tests, 146/146 runnable Kujo fixtures, and `cargo audit --deny warnings` passed. |
| `bash .github/scripts/check-release-state.sh` | PASS | Prepared source version 1.2.0 remains distinct from published stable tag v1.1.0. |
| `cargo metadata --locked --no-deps --format-version 1` | PASS | The locked package metadata reports `kujolang 1.2.0`. |
| Loop Engineering configured gates | PASS | Release-state, focused routed-HTTP, setup-action, minimal-release, and clean-diff gates passed; the loop remains partial only because tagging, artifact publication, and post-publication stable-string updates are release-pipeline steps. |

Warnings and exceptions:

- `cargo-deny` is not installed on the verification host, so the canonical gate skipped that optional command. The required RustSec audit passed with warnings denied.
- The release-candidate gate does not run the optional benchmark smoke unless `KUJO_RELEASE_GATE_RUN_BENCH=1` is set; no release claim depends on that optional benchmark.
- The repository-required `UNBLOCK_V1_RELEASE` directive was supplied in the active release-execution context.

Release readiness sign-off:

The v1.2.0 source is ready for a signed release tag after the release-preparation commit is pushed and its hosted required checks pass. Publication is not complete until all platform assets, checksums, and the published-artifact smoke workflow succeed.

## Follow-ups / TODO (For Future Agents)

- Create and verify the signed v1.2.0 tag after hosted checks pass.
- Wait for all four platform artifacts, consolidated checksums, and the
  published-artifact smoke workflow before updating stable-release strings.
- Migrate downstream repositories to the checksum-verified setup action.

## Links / References

- Release process: `docs/RELEASE_PROCESS.md`
- Candidate gate: `scripts/release_candidate_gate.sh`
- Setup action: `.github/actions/setup-kujo/action.yml`
