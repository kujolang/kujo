# Native fuzz reproducibility

Use `scripts/fuzz_smoke.sh` and `scripts/fuzz_repro.sh` from the source revision
being tested. Both read the dated compiler pin in `fuzz/RUST_TOOLCHAIN` and the
exact cargo-fuzz version in `fuzz/CARGO_FUZZ_VERSION`. Install those versions;
the scripts do not install or update tools automatically. CI uses the same pins.

The fuzz package is a separate workspace. Its committed `fuzz/Cargo.lock` is
independent of the root lockfile and includes the reviewed vendored HTTP patch.
Before execution, `cargo metadata --locked` validates dependency resolution.
The receipt prints its SHA-256; each successful run then checks that it is
unchanged. cargo-fuzz 0.13.2 has no `--locked` run option, so these are explicit
preflight and post-run checks, not a claim about an unsupported cargo-fuzz flag.

Run `bash scripts/fuzz_smoke.sh --check-prereqs` before a campaign. Actual runs
also require a valid dependency lock. Use `--run-root` to retain copied seed
corpora and crash artifacts outside the source tree. Reproduction `--dry-run`
remains read-only and shows the exact pinned compiler command.

To update intentionally, review the compiler/cargo-fuzz pins and regenerate
`fuzz/Cargo.lock` with the selected compiler, then run the script contract tests
and bounded native targets. Commit the new pins, lock, and evidence together.
Record source SHA, compiler version, platform, sanitizer, native compiler and
build flags alongside the campaign. These pins do not make cross-platform
executables byte-identical or prove production safety.

The native campaign still measures only the bounded inputs it executes.
Cold sanitizer compilation can dominate elapsed time; libFuzzer's input-runtime
limit does not cap the build phase. Preserve raw build and target output.

## 2026-09-12 acceptance

The five runner contract tests and Rust formatting pass. A Linux x86-64 address-
sanitizer campaign used nightly-2026-09-05, cargo-fuzz 0.13.2, one compiler job
and 20 seconds configured per target. Source was frozen at `d054d87` plus this
fuzz-runner slice; all 151 recorded input hashes remained unchanged. The fuzz
lock SHA-256 was
`954c1ee5ac15635159eb90d1f58577c8b0016fe8e477b79e7a1220ef3126151b` before and
after execution.

| Target | Executions | Peak RSS, MB | Crash artifacts |
| --- | ---: | ---: | ---: |
| lexer | 221630 | 472 | 0 |
| parser | 92068 | 459 | 0 |
| xml_bounded | 3783 | 443 | 0 |
| gzip_bounded | 2048 | 429 | 0 |
| zip_single_bounded | 2179 | 428 | 0 |

All targets exited successfully, with no timeout or OOM failure. This bounded
smoke campaign is not an exhaustive security or capacity result. Earlier host
process-exhaustion and Linux disk-exhaustion builds failed before a campaign
completed; they remain failures. Disposable compiler caches were removed before
the successful frozen-source run, without changing its targets or limits.

Reproduce with `bash scripts/fuzz_smoke.sh --max-total-time 20 --run-root PATH`.
Detailed receipt: `.muzzle/reports/email-fuzz-acceptance-20260912.json`;
source manifest: `.muzzle/reports/email-fuzz-frozen-20260912.json`;
raw log: `.muzzle/logs/email-fuzz-linux-frozen-20260912.log`;
runner tests: `.muzzle/logs/email-fuzz-contract-cargo-20260912.log`.
The retained corpora/artifact directories are under
`/var/tmp/email-fuzz-frozen-20260912` in the existing Linux verification VM.
