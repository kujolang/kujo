# Bincode maintenance remediation — 2026-10-19

Repository: `kujolang/kujo`, branch `main`.
Starting revision: `69e390537612bd725519089724cab341f73720a5`.
Implementation revision: `17f582f7d0cf4987b1d1cf65c073e634a4571105`.
Ending revision is the documentation commit containing this receipt.

The owner requested removal of the remaining maintenance warning while preserving
compatibility, full verification, commits and push. The starting revision's
[ZIP migration receipt](zip9-migration-2026-10-19.md) records 3,112 passing tests
and the sole remaining bincode exception.

## Finding and narrow change

| ID | Priority | Finding | Action |
| --- | --- | --- | --- |
| BIN-1 | P2 | Retired bincode 1.3.3 via printpdf → azul-layout → hyphenation 0.8.4 triggers RUSTSEC-2025-0141 | Replace its normal and build codec dependency; remove the release-gate exception |

This is a maintenance advisory, not a demonstrated exploitable vulnerability.
Contrary to the previous gate comment, bincode was used both at build time and
at runtime for dictionary loading. Hyphenation 0.8.4 remains the latest registry
release checked during this task. Disabling hyphenation or changing the PDF
renderer would risk changing layout. Instead, a reviewed local patch follows
the repository's existing vendored-dependency convention.

The vendored hyphenation retains the original algorithms, pattern sources,
licenses and all 82 binary dictionaries. Only its normal/build codec dependency,
codec calls and codec error payload types change. Bincode-next 2.1.0 uses explicit
`config::legacy()` with serde: little-endian fixed-width integers, preserving
bincode 1's dictionary format and reader consumption. Only `std`/`serde` features
are enabled. Version 2.1 is pinned because its Rust 1.85 minimum fits Kujo 1.89;
latest bincode-next 3.1.1 requires Rust 1.90. No retired code was renamed to hide
an advisory. The root lockfile removes bincode, adds bincode-next and unty-next,
and changes hyphenation to the local patch without updating unrelated packages.

## Compatibility evidence

Before patching, a retained executable linked against original hyphenation and
bincode captured SHA-256 hashes of complete recursively canonicalized JSON
representations of every decoded dictionary. This covers dictionary language,
minima, FST data and exceptions, not merely a few sample words. The generator,
baseline and locked compatibility fixture are committed.

The replacement reproduces all 82 hashes through both reader and embedded paths.
Its encoded dictionaries were then read by the retained original executable;
all 82 content hashes matched again. HashMap order is intentionally not claimed
to be byte-for-byte stable. Encoded lengths, truncation rejection, language
mismatch, trailing-byte reader boundaries and English hyphenation are checked.
The optional build-from-patterns path reproduces the same embedded dictionary
content. Both fixture modes also pass on Kujo's declared Rust 1.89 minimum.

Kujo's public Rust APIs, language builtins, PDF output contracts, CLI exit codes,
config, schemas and persisted formats remain unchanged. Kujo does not expose
hyphenation's internal codec error types. The patched third-party crate's error
payload type differs; this is documented in its README and is not a supported
standalone replacement API claim for external hyphenation users. Existing Cargo
root patch limitations remain: patches are not propagated from a library into
an unrelated downstream root manifest. No sibling repository was modified.

There are no performance claims. The tradeoff is maintaining the narrow
hyphenation patch until a suitable upstream release is available. This is an
owned dependency patch, not an unresolved blocker.

## Verification receipt

Cargo runtime checks used `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0`, `--locked` and `-j1`; full tests additionally set
`KUJO_ENABLE_SOCKET_TESTS=1`.

| Command | Result |
| --- | --- |
| `cargo test --locked --no-fail-fast -j1` | 3,112 passed across 121 targets; zero failures; 20 existing ignores |
| `cargo clippy --locked --all-targets --all-features -j1 -- -D warnings` | Passed |
| `cargo check --locked --no-default-features -j1` | Passed; 17 existing feature-disabled warnings |
| `target/debug/kujo test --runtime vm` and `--runtime dual` | Each 150/150; 11 existing policy skips; zero dual fallbacks |
| `cargo run --locked --manifest-path tests/fixtures/hyphenation_compat/Cargo.toml` | All 82 dictionary comparisons and boundary checks passed |
| Same fixture with `--features rebuild` | Passed for build-time serialization and embedded loading |
| Both fixture commands with `cargo +1.89.0` | Passed on declared minimum Rust version |
| `cargo clippy --locked --manifest-path tests/fixtures/hyphenation_compat/Cargo.toml --all-features -- -D warnings` | Passed |
| Original-codec baseline executable on replacement-encoded dictionaries, then `cmp` with baseline.json | All 82 dictionaries match exactly in canonical decoded content |
| `cargo audit --deny warnings --json` | Passed with no exceptions: zero vulnerabilities, zero warnings |
| `cargo audit --deny warnings --file tests/fixtures/hyphenation_compat/Cargo.lock --json` | Passed with no exceptions |
| `cargo fmt --check`, standalone fixture/generator `rustfmt --check`, `git diff --check` | Passed |
| `bash -n scripts/release_gate.sh`, Ruby YAML parse of new workflow, `bash scripts/repo_hygiene_audit.sh` | Passed |

Fixture builds used separate `CARGO_TARGET_DIR` directories and disabled dev
debug information to bound disk usage. Existing upstream dependency compiler
warnings are distinct from the now-clean dependency maintenance audit.

The first long full-test invocation exited 101 after 23 completed targets with
no failing assertion or final diagnostic in its log. The cause was not
established. The complete cached rerun passed all 121 targets, including the
diagnostics target where that log ended. No assertion, timeout or test policy
was changed to obtain the pass. Both logs are retained.

Host execution is macOS x64. A committed CI matrix verifies dictionary loading
and rebuilding on Linux/Rust 1.89 and macOS/Windows stable. Remote matrix results
are not claimed as local evidence. The repository's existing full runtime and
PDF regression tests remain enabled.

Detailed local logs are retained under
`.audit-evidence/bincode-maintenance-2026-10-19/`.

## Sources

- [RustSec maintenance advisory](https://rustsec.org/advisories/RUSTSEC-2025-0141.html)
- [Hyphenation source](https://github.com/tapeinosyne/hyphenation)
- [Bincode-next maintained project](https://github.com/Apich-Organization/bincode)
- [Selected release](https://crates.io/crates/bincode-next/2.1.0)
- [Latest release metadata](https://crates.io/crates/bincode-next/3.1.1)
