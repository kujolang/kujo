# Kujo Field Notes — v1.3.1 Linux Upgrade Reliability

**Date:** 2026-09-06
**Session:** 11:47 local
**Branch/Commit:** codex/release-v1.3.1 / 987314cd431bcea14a0bc473387b74ae9eccb1d0
**Scope:** npm v1.3.0 recovery and a narrowly scoped Linux runtime patch

---

## What I Changed

Restored publication of all six npm v1.3.0 packages after the user configured
trusted publishers. Registry inspection found none before the retry; only the
failed npm job was rerun. Release run 33991002720 attempt 3 and clean native npm
installation matrix 34039905245 passed. The signed v1.3.0 tag and its 11 GitHub
assets remain unchanged.

Prepared v1.3.1 from the released v1.3.0 source. Linux staged executable checks
retry only ETXTBSY within one deadline shared with process/output validation.
Other errors, SHA-256 verification, version matching, destination identity,
ownership checks, and backups remain enforced. This changes runtime behavior,
so the user's explicit conditional patch-release request authorizes v1.3.1.
Unrelated later runtime features on main are excluded from the patch release.

## Gotchas (Read This Next Time)

A concurrent child can retain a writable executable descriptor after the parent
closes its own handle. O_CLOEXEC closes that descriptor at exec, not fork. The
controlled regression identifies the child descriptor with /proc and proves its
writable access mode. Releasing that exact descriptor allows the same bytes to
execute. This matches rust-lang/rust#114554 and Linux execve's ETXTBSY contract.

The original historical CI run has no syscall trace; its precise child PID is
unknown. Forty ordinary unfixed concurrent runs did not spontaneously reproduce
it. Controlled tests against the unfixed implementation did reproduce ETXTBSY,
including the actual staged replacement path, while preserving the installed
binary. Do not describe the evidence as an observed PID from the historical run.

## Things I Learned

A shell readiness printf with temporary stdout redirection can itself race /proc
inspection. The final fixture only waits on stdin; successful spawn supplies the
exec handshake and stdout remains the retained writable descriptor throughout.

## Debug Notes (Only if applicable)

- Controlled red: both the direct staged check and replacement-and-backup
  regression failed against the unfixed implementation with OS error 26.
- Corrected green: 40 consecutive Linux runs of all 17 non-self-copy upgrade tests
  at 16 threads passed, followed by the complete 18-test suite including running
  executable replacement (90.23 seconds). A preliminary 17-test run also passed.
- Full hosted release gate: 34041764048 passed, including fmt, Clippy, minimal
  release smoke, parity and the full gate.
- Native matrix: 34041762327 passed on all five supported platforms. Linux x64
  and ARM64 each passed the initial full 18-test suite plus 20 complete parallel
  repeats at 16 threads; both macOS targets and Windows also passed native
  replacement, minimal-feature compile and upgrade CLI contracts.
- npm resolver tests and pack dry run passed; Cargo, all npm manifests and the
  proposed tag agree on 1.3.1. Release-state and contract-version guards passed.
- ShipCheck scan/gate against kujo-release-v1.3.1: exit 0, 12/16 checks, zero
  errors, four documented Cargo-detector warnings; gate accepted.
- Detailed local Linux receipts: /tmp/kujo-linux-race-evidence.tgz. The task
  container was removed and Colima restored to its initially stopped state.

## Follow-ups / TODO (For Future Agents)

Publish the signed patch tag only after native validation is complete. Verify
all GitHub assets, six npm packages, clean native installs and published binary
smoke before promoting stable-version references. No Cargo publication is
included because normalized crates lose the local hardened tiny_http patch.

## Links / References

- https://github.com/kujolang/kujo/actions/runs/34041764048
- https://github.com/kujolang/kujo/actions/runs/34041762327
- https://github.com/kujolang/kujo/actions/runs/34039905245
- https://github.com/rust-lang/rust/issues/114554
- https://www.man7.org/linux/man-pages/man2/execve.2.html

Release-readiness sign-off: all required candidate gates passed. The candidate
is ready for its signed v1.3.1 tag; publication and artifact smoke are separate
required checks. Main integration 740c3b0 also passed full gate 34042085707.
