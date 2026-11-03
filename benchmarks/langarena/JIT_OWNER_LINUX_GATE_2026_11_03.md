# Initial exclusive code-owner boundary: Linux gate

Exact candidate `e34f5925f520ffa11db00b6accaede07d02952f2` passes all seven
jobs in [run 41626686442](https://github.com/kujolang/kujo/actions/runs/41626686442):
formatting, Clippy, Rust 1.89 MSRV, VM/interpreter parity, minimal release smoke,
full release checks, and optimized library/loop/callback/error contracts.

The optimized artifact records **1,360 passes and eight existing manual ignores**:
1,321 library passes and 39 tests across eight integration targets, with zero
failures or filtered tests. Source commit and JIT/loop-test hashes match pinned
Git blobs. Cargo records exactly the library and requested eight test binaries.
All 39 integration names match the pinned test sources. The four owner-lifetime
tests present at this revision pass, including transfer to a fresh VM on another
thread and Linux executable-entry revocation. Toolchain: Rust 1.99.1, Linux x64.

The full release gate includes repository fixture runs with 150 passes and 11
skips in both dual and interpreter strategies. **Those are not the LangArena
150-case matrix.** Optional benchmark smoke, `cargo audit` and `cargo deny` were
not run; the latter two tools were unavailable in the job.

`probes/verify-owner-linux-gate.rb` independently checks run/job outcomes, source
identity, exact targets, integration membership, owner tests, ignored-test
membership, summaries and build outcome. Its six controls accept the original
evidence and reject changed source, a failed job, duplicate owner/integration
membership and filtered tests. This parser is scoped to these directly annotated
integration tests; it is not a general Rust test-discovery implementation.

`jit-owner-linux-gate-e34f5925-2026-11-03.json.gz` retains the API records, seven
job logs, optimized source/test artifact, verifier, controls and verification
receipt with per-file SHA-256 digests.

This validates the initial ownership-bundle boundary only. It does not accept
the later compilation/metadata accounting, task reuse, all-platform lifetime,
all-150 LangArena or production gates, or any speed improvement. The later
`cb931728` Linux lifetime job passed six owner tests but failed its separate
no-default-features build; that failure and the pending feature-guard correction
are recorded in `notes/JIT_OWNER_REUSE_CANDIDATE_2026_11_03.md`. Do not substitute
this earlier green run for the newer candidate's outstanding checks.
