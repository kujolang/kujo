# Kujo builtin performance hardening

## Repository

This pass covers native functions shared by Kujo's Rust bytecode VM and
tree-walking interpreter. It starts from clean `kujolang/kujo` `main` at
`28af8e70b450cb34d2c8967726f7ffae9d2ebf91`. The preceding
[function-dispatch pass](repository-hardening-2026-11-22-function-speed.md)
is part of this baseline. The verified code ends at
`13c4412b7469b2c2f6835af99dc0edd516077e4b`.

## Findings

| ID | Priority | Builtin | Evidence and change | Status |
| --- | --- | --- | --- | --- |
| 1 | P1 | `Set` | Previously scanned the growing result for each input. Large supported scalar inputs now use a temporary membership index and retain first occurrence order. | Fixed |
| 2 | P1 | `set_union` | Previously searched the growing left-to-right result for every right item. The index starts with left-side membership and grows only as new entries are appended. | Fixed |
| 3 | P1 | `set_intersect` | Previously searched the right side for every left item. Large supported scalar inputs index the right side and walk the left in order. | Fixed |
| 4 | P1 | `set_difference` | Previously searched the right side for every left item. The same indexed lookup retains left values absent on the right. | Fixed |
| 5 | P2 | `vec_top_k` row validation | Constructed a temporary indexed argument name for every valid row. Format the row label only when a validation error uses it. | Fixed |
| 6 | P1 | `vec_top_k` scoring | Divided every query component by the same query scale for every row. Scale the already-owned query buffer once and reuse it for all rows. Zero-query scoring skips row norms after validation. | Fixed |
| 7 | P2 | `vec_top_k` result construction | Allocated two identical dictionary key strings per returned row. Reuse the two immutable keys across the result. | Fixed |
| 8 | P1 | `escape_xml` | Copied the input string and allocated an output even for text without delimiters. Borrow input text and reuse its string storage when escaping leaves it intact. | Fixed |
| 9 | P1 | `render_listing_card` | Copied arguments, formatted terms beyond the three displayed, and escaped the title twice. Borrow strings, format displayed terms only, and reuse the escaped title. | Fixed |

The ordered-set implementation is in `src/interpreter/native_functions/collections.rs`.
The index is restricted to integer, Boolean, string and null values.
Floats and compound values keep the ordinary equality path. This preserves
approximate float equality, cross-type numeric comparisons, and first-match
behavior even when numeric comparisons are nontransitive. Indexed membership
never controls iteration order.

In `src/interpreter/native_functions/vector.rs`, query scaling preserves the
arithmetic order of every row's reduction. Regression coverage compares raw
binary64 score bits on zero, signed-zero, maximum finite and subnormal inputs.
The existing tests cover indexed error messages, matrix validation with
`k = 0`, result shape, partial selection and tie order.

The XML and listing helpers in `src/interpreter/native_functions/strings.rs`
retain the original five-entity escaping for both HTML text and attributes.
New tests exercise multibyte text, non-string coercion, escape characters in
a listing title and term, and omission of terms beyond the first three.

## Measurements

The host was macOS 26.6.2 x86_64 with an Intel Core i7-9750H. Each benchmark
used one warm-up pair and five timed pairs in alternating binary order. Each
sample is fresh-process user plus system CPU time, including startup and
fixture setup. The table shows the median and observed range in milliseconds.
The harness checked the expected aggregate stdout and the same stderr on
both binaries. The two range sets overlap by 1,500 values in each set
operation workload. Each timing is a whole workload measurement.

| Workload | Baseline CPU ms | Final CPU ms | Median change |
| --- | ---: | ---: | ---: |
| 6 constructions over 6,000 distinct integers | 1,911.83 (1,881.01–1,968.70) | 68.24 (67.94–73.51) | -96.4% |
| 12,000 constructions over 4 values (control) | 594.26 (584.94–604.05) | 584.85 (575.60–601.70) | -1.6% |
| 6 unions of overlapping 3,000-item integer sets | 1,059.90 (1,055.85–1,098.71) | 75.66 (72.38–78.18) | -92.9% |
| 6 intersections of overlapping 3,000-item integer sets | 697.34 (672.10–709.25) | 65.34 (61.78–67.19) | -90.6% |
| 6 differences of overlapping 3,000-item integer sets | 671.66 (666.88–675.62) | 63.04 (59.50–64.53) | -90.6% |
| 45 top-3 calls, 768 rows × 384 dimensions | 2,110.92 (2,093.80–2,114.12) | 2,101.47 (2,030.91–2,109.49) | -0.4% |
| 1,000 top-3 calls, 511 rows × 3 dimensions | 806.51 (798.78–816.60) | 652.38 (638.91–655.00) | -19.1% |
| 500 full rankings, 511 rows × 3 dimensions | 898.63 (895.56–917.02) | 695.42 (688.98–711.16) | -22.6% |
| 450 XML escapes of 54 KiB UTF-8 text without delimiters | 576.17 (573.68–587.49) | 218.54 (216.45–222.22) | -62.1% |
| 450 XML escapes of UTF-8 text containing all five delimiters | 927.97 (902.97–932.60) | 926.18 (900.86–927.77) | -0.2% |
| 1,500 listing cards with long titles and excerpts | 410.20 (407.67–412.84) | 222.36 (218.18–228.08) | -45.8% |

The small-set control, 384-dimension ranking, and entity-heavy XML escape
had overlapping timing ranges. The three-dimensional ranking and full result
workloads showed lower CPU use. Their timings combine query scaling, label
formatting and result construction, so they do not isolate those changes.
The set operations include constructing their two input sets, which also
benefits from the indexed constructor.

## Audit coverage

| Phases | Evidence and scope |
| --- | --- |
| 0–1 | Checked the clean branch and prior function-speed report. Traced VM and interpreter native dispatch to collection, vector and SSG string handlers. Inspected Cargo features, build profile, verification contracts and the release CI gate. Captured the baseline suite and a runnable CLI before editing. |
| 2–4 | Found nested membership scans, per-row repeated work and avoidable ownership in strings and vector results. Compared process CPU time. Allocation reductions follow from the code paths. Peak RSS was not measured. |
| 5–6 | Vector scoring runs locally and does not interact with tool registration, model prompts, retries, replay or handoffs. The CLI process emitted matching output in each timed pair. Agent context and generated report volume were outside scope. |
| 7–9 | Kept matrix validation order and numeric limits, deterministic set and score ordering, existing Rayon partitioning and escaping rules. The selected calls do not introduce I/O, subprocesses, shared mutable state or capability changes. |
| 10–11 | Signatures, CLI options, payloads, configuration, file formats and dependencies are unchanged. |
| 12–14 | Added regressions for ordered sets, numeric fallback, bitwise cosine scores, XML escaping and listing cards. Updated the standard library and changelog. Timings are observations without a CI wall-clock threshold. |
| 15–16 | Kept the existing entrypoints and diagnostics for scripts and agents. Repository-wide tooling, dead code and release packaging were outside this builtin speed pass. |
| 17–18 | Verified the implemented paths using the commands and paired measurements below. |

No cross-repository change is required. The VM, JIT and interpreter call-frame
improvements from the previous report remain the baseline for this pass.

## Verification

Both builds used Rust 1.96.0, the locked default features, and the same debug
profile with `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3`. The baseline matrix ran the
first seven named integration suites and the library. The baseline passed 1,790 tests and the final matrix passed 1,799, both with
9 ignored. The final matrix included the schema and vector contracts.

```sh
cargo fmt --check
git diff --check
cargo test --locked --no-fail-fast --lib --test vm_interpreter_parity_surfaces \
  --test interpreter_tests --test cli_contracts --test cli_json_contracts \
  --test diagnostics_golden --test docs_examples --test readme_contracts \
  --test schema_vector_hardening_contracts
target/debug/kujo test --runtime vm
target/debug/kujo test --runtime dual
```

A separate compatibility probe compared baseline and final stdout byte for byte
through both `kujo run` and `kujo run --interpreter`. It covered mixed ordered
sets, tied and zero cosine scores, and the XML and listing output. Both fixture
modes passed all 150 runnable fixtures with 11 inventory skips, and dual mode
reported zero interpreter fallbacks. The first candidate test run used an
incorrect expected `alt` string in the new listing test. The assertion was
corrected to include the established `Featured image for` prefix.

The unrestricted debug test attempt was replaced by the matrix to limit disk
use. An optimized standalone trial for `str_reverse` preallocation produced
overlapping samples, so that change was dropped before final verification.
Optimized whole-runtime timing, peak RSS and throughput on other machines
were outside this pass.

The probe and timing harness live in `/tmp/kujo-builtin-speed/` on the
measurement host.

```sh
python3 /tmp/kujo-builtin-speed/bench.py /tmp/kujo-builtin-speed/baseline-kujo target/debug/kujo
zsh /tmp/kujo-builtin-speed/finish-verification.zsh
```
