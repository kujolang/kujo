# JIT verification fixture corrections

These are failed verification rounds, not accepted runtime or performance results.
The accepted runtime remains `fd7b764`; neither correction changes runtime code.

## Direct compiler fixture

On candidate `34bc5d17fdf1a9cc226e54bf0dd486ba1a072a09`,
[the loop regression job](https://github.com/kujolang/kujo/actions/runs/41265318868/job/124726820712)
passed 1,310 library tests (eight ignored) and 31 integration tests. One direct
compiler test failed: its top-level loop contained `PushScope`, which native
loop admission rejects. The test bypassed that admission check. All seven
runtime-based condition-call tests and all 15 collection-loop tests passed.

Correction `6d96c151e8b9f3d814043eebe5768ae5c61cde97` extracts a
function-local loop from compiler output, asserts the VM's admission predicate
and the presence of `Call(1)`, then requires direct compilation. It does not
remove or rewrite opcodes. A focused standalone compilation probe passes, but
the complete corrected local and remote gates remain pending.

## CLI warning fixture

On compatibility candidate `e67a936f26d464c2fef66ec77944c1004b08840a`,
[the full release gate](https://github.com/kujolang/kujo/actions/runs/41265321279/job/124728414564)
failed `cli_run_with_jit_opt_in_reports_unsupported_regions_without_disabling_jit`.
Its array literal/index fixture is now supported, so expecting a fallback warning
is stale. This failure occurred after the Linux SSG executable-start regression
passed; it is distinct from the earlier `ETXTBSY` failure.

Correction `d0f9e9af743bd7d2f08447dc3399bcdebd23b8c1` uses a real
unsupported `DefineGlobal` path, requires that specific diagnostic and successful
execution, and moves array checks into the supported-surface test with an explicit
no-warning assertion. All four execution-contract tests pass against the frozen
accepted executable (SHA-256
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`).
The standard-library-only Rust test suite was built directly with `rustc --test`;
this focused result does not replace the full release gate.

## Preserved evidence and remaining gates

The [compressed bundle](jit-fixture-failures-2026-10-31.json.gz) preserves 11
artifacts: exact run/job metadata, loop source/toolchain receipt and complete test
log, full compatibility release log, focused CLI test results and the diagnostic
fixture/output. Every artifact was decoded, matched to its original bytes and
verified against its SHA-256. All failure records remain failures.

New runs `41267049672` (loop correction) and `41266978042` (compatibility
correction) were dispatched separately. Full regression acceptance, all 150
test-size checksum checks for the corrected candidate, and before/after loop
measurements remain required. No speedup is inferred from these fixture repairs.
