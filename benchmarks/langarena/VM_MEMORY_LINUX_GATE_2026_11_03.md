# Memory candidate: Linux release-gate evidence

[Workflow 41611840423](https://github.com/kujolang/kujo/actions/runs/41611840423)
passes all seven jobs on exact candidate commit
`754453061a98fe3c994b89f1c9308326a50c544d`:

- formatting;
- Clippy across all targets/features with warnings denied;
- locked MSRV check (Rust 1.89);
- VM/interpreter parity, including the workflow's no-JIT feature configuration;
- minimal release smoke;
- full release gate;
- optimized library and eight loop/callback/error integration suites.

The last job records **1,358 passing tests and eight existing ignores**, with
zero failures or filtered tests. Its nine successful test summaries and passing
test-name count agree; Cargo's JSON artifacts identify exactly the library and
eight requested targets. The recorded source commit and JIT/loop-test SHA-256
hashes match their committed blobs. Toolchain: Rust 1.99.1, Linux x86-64,
LLVM 23.1.1. No local binary is substituted for this CI build.

Both code-owner semantic tests run: returned values/functions/snapshots survive
the original VM, and disabling JIT retains cached code. **The Linux executable
mapping assertion is not enabled at this candidate revision.** That test was
added later on the separate platform-verification branch; its matrix is still
pending. This Linux workflow alone does not prove reclamation on every platform.

The full release log retains successful repository fixture runs, each reporting
150 passed and 11 skipped. Those are repository fixtures, not LangArena production
results. Optional benchmark smoke, `cargo audit` and `cargo deny` were not run;
the latter tools were unavailable in that job. Existing ignored tests remain
visible in the logs. No release artifact publication occurred.

`vm-memory-linux-gate-2026-11-03.json.gz` preserves the run/jobs API records,
seven job logs, optimized test artifact/source receipt, source snapshots and
verification summary. Every embedded artifact is SHA-256 checked after writing.

The memory runtime remains separate and unaccepted. Remaining gates include the
in-progress production controls, Linux/macOS architecture lifetime matrix and
Windows lifecycle build/tests. Windows backend reclamation remains a distinct
known limitation. This report makes no speed or RSS improvement claim.
