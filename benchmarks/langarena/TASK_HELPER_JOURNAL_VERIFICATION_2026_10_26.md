# Ordinary helper capture-journal verification

Accepted runtime: `15a225d`; baseline: `45360b9`. Helper-local parameter scopes
previously reused suspended captured-local scope indexes. Their writes could
therefore overwrite another task's unrelated captured write. Global helper entry
now restricts the active journal to globals and restores its original scope limit
on return, including nested helpers and caught errors. Actual globals and later
captured writes remain tracked; value semantics are unchanged.

The focused suite reproduces four interpreter failures on the baseline (output
1 instead of 2), with eleven passing controls. All fifteen cases pass afterward:
five fixtures across VM, interpreter and JIT-enabled execution. A unit test checks
nested helper parameters, real global writes, retained captured writes and a
separate owner's untouched value.

## Verification

- Release build, `cargo check --locked -j2`, formatting and diff checks pass.
- 269 integration tests pass, one ignored; 1,115 library tests pass, eight ignored.
- All 50 test-size LangArena checksums pass in each runtime: 150 successes.
- Eight runtime/test source hashes match the accepted measurement manifest.

[Machine-readable evidence](task-helper-journal-verification-2026-10-26.json)
contains exact fixtures, hashes, test logs, checksum results and all 96 measurement
observations. Local raw evidence remains in
`.audit-evidence/task-helper-journal-2026-10-26/`.

## Local performance gate

Seven alternating pairs per fixture/runtime, measured after compilation and
correctness checks. Ratios are candidate elapsed time divided by baseline;
smaller is faster. All observations, including slow pairs, are retained.

| Fixture | VM median | Interpreter median | JIT-enabled median |
| --- | ---: | ---: | ---: |
| Ordinary global helper, 262,144 calls | 1.03758 | 0.98465 | 1.01939 |
| Imported async helper, 65,536 calls | 0.98382 | 0.97407 | 0.97592 |

All six medians pass the 1.05 slowdown gate. Imported async-helper medians improve
about 1.6–2.6%; every pair in those three groups is below 1. Ordinary VM calls
are 3.76% slower at the median, so this is not a general speed-win claim.
Individual ordinary-helper ratios span 0.772–1.204 on this shared laptop.

Twelve separate peak-RSS observations are retained. Async interpreter RSS is
8,175,616 → 8,531,968 bytes; other observations vary slightly or remain unchanged.
Single RSS observations do not establish retained-memory trends. No allocation,
native-JIT-entry, production-size, clean-host or Goose/Rust comparison claim is
made by this experiment.

## Still open

Ordinary task whole-snapshot publication can lose separate captured writes;
captured task globals can be stale; ordered task/caller capture reads can be
stale. These require separate ownership fixes and are not closed by this journal
scope correction. Remaining imported binding materialization and broader profile
work stay unchecked in the performance checklist. Clean-host comparisons remain
pending because no dedicated host is available.
