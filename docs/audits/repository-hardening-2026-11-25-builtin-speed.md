# Kujo builtin performance hardening

This pass covers the Rust native functions used by the bytecode VM and
tree-walking interpreter. It started on clean `kujolang/kujo` `main` at
`899de3cd312ebce63120c380264e66c6826e402d`. The verified implementation is
`3957383d3118140f3903600310e40a51ceaabab2`. The [previous builtin audit](repository-hardening-2026-11-22-builtin-speed.md)
already covered sets, vector ranking, XML escaping, and listing cards.

## Findings

| ID | Priority | Builtin | Evidence and change |
| --- | --- | --- | --- |
| 1 | P1 | `insert` | Validated after cloning, then grew the copy and shifted its tail. Validate first and allocate the result at its final size. |
| 2 | P1 | `remove` | Copied the array before searching it. Search the source, share it on misses, and copy only retained elements on hits. |
| 3 | P1 | `remove_at` | Copied and shifted the entire array, including on invalid indexes before validation. Validate first, then copy the retained slices and return the removed element. |
| 4 | P2 | `values` | Sorted hash-dictionary keys, then hashed each key again to recover its value. Sort entry references by key and copy the selected values. |
| 5 | P2 | `items` | Repeated the same key-only sort and lookups when assembling pairs. Use the sorted entry references. |
| 6 | P1 | `truncate` | Counted the entire input for short previews and constructed an intermediate prefix. Check the length only as far as needed, then copy the prefix and suffix into one buffer. Share the source when it fits. |
| 7 | P2 | `char_at` | Scanned from the beginning on every lookup. Reuse the existing bounded string metadata cache for deep ASCII lookups and index those bytes directly. |
| 8 | P1 | `slugify` | Materialized a transformed string, collected segments, joined, replaced, and copied again. After lowercase conversion, write the result in one pass and emit separator runs only between kept characters. |

Array validation and exact scalar matching are shared with the public Rust
helpers in `src/builtins.rs`. The arrays remain value-like under copy-on-write.
Hash dictionaries retain lexicographic key order and all dictionary
representations retain their existing result shape. Unicode input retains
character-based indexing, contextual lowercase conversion, and the established
suffix and separator behavior. The cache is thread-local, bounded, and holds
weak references, using the existing invalidation behavior for string mutation.

The added VM/interpreter parity test exercises successful, empty, and
out-of-range operations, numeric coercion, source preservation, dictionary
ordering, multibyte boundaries, and contextual lowercasing. The standard library
entry documents existing truncation and slugification rules more precisely.

## Measurements

The host ran macOS 26.6.2 x86_64 on an Intel Core i7-9750H with Rust
1.96.0 and the locked default features. The paired binaries used the same
unoptimized Cargo test profile with debug info and incremental compilation
disabled. One warm-up pair preceded five timed pairs in alternating binary
order. Each sample includes CLI startup and fixture setup. The harness
asserted expected stdout and matched stderr. It uses Python's standard
process accounting to sum child user and system CPU time. The table shows the
median and observed range in milliseconds.

| Workload | Baseline CPU ms | Candidate CPU ms | Median change |
| --- | ---: | ---: | ---: |
| 220 inserts in 16,000-element arrays | 915.92 (871.79–3,490.76) | 772.05 (759.35–973.66) | -15.7% |
| 220 unsuccessful removals in 16,000-element arrays | 1,700.79 (1,489.57–1,730.93) | 569.48 (457.68–584.90) | -66.5% |
| 220 successful removals in 16,000-element arrays | 1,454.78 (1,300.08–1,499.63) | 1,150.39 (700.27–1,272.24) | -20.9% |
| 220 indexed removals in 16,000-element arrays | 1,443.41 (1,264.80–1,706.42) | 1,136.52 (1,039.74–1,201.99) | -21.3% |
| 45 value listings of 2,048-entry hash dictionaries | 557.15 (547.80–564.93) | 455.42 (449.48–462.82) | -18.3% |
| 45 pair listings of 2,048-entry hash dictionaries | 912.52 (904.95–936.63) | 803.56 (786.63–911.79) | -11.9% |
| 220 long UTF-8 slices (rejected experiment) | 1,293.65 (1,267.87–1,374.32) | 1,620.33 (1,576.56–1,687.96) | +25.3% |
| 600 short previews of 128 KiB UTF-8 text | 206.40 (195.46–225.56) | 166.01 (155.97–172.24) | -19.6% |
| 600 truncations where the text fits | 219.47 (211.24–232.56) | 156.13 (141.68–158.29) | -28.9% |
| 1,400 deep indexes of a 64 KiB ASCII string | 4,690.64 (4,581.85–5,351.93) | 204.74 (167.72–253.49) | -95.6% |
| 4,000 indexes of a short string (control) | 517.52 (500.70–526.83) | 520.98 (476.67–538.99) | +0.7% |
| 140 slug conversions of repeated mixed-Unicode text | 2,129.81 (2,122.53–2,232.93) | 826.21 (815.06–849.76) | -61.2% |

The short-index control had overlapping ranges. The long Unicode
`substring` experiment increased CPU time by 25.3% and was reverted. The
other timed scripts exercised the same builtin implementations retained in
the verified build. The candidate timings preceded the isolated substring
revert and lint annotations. Release-profile timings and peak RSS are
unmeasured. The [harness](evidence/kujo-builtin-speed-2026-11-25.py) and
[raw samples](evidence/kujo-builtin-speed-2026-11-25.jsonl) preserve the inputs
and observed variability.

## Audit coverage

| Phases | Evidence and scope |
| --- | --- |
| 0–1 | Checked the initial Git state and prior audits, followed native dispatch from the VM and interpreter, inspected the relevant Rust library exports and Cargo profiles, and ran the baseline matrix and CLI probes. |
| 2–4 | Reviewed traversals, source borrowing, result allocation and cache behavior in the changed paths. Measured process CPU time. Allocation effects follow from the implementation. Peak RSS was outside scope. |
| 5–6 | These native calls do not construct tool schemas, prompts, AI replay or retry traffic, or handoffs. Timed scripts checked the expected stdout. Agent context budgets and other CLI output were outside this pass. |
| 7–9 | Kept validation and error text, UTF-8 boundaries, deterministic ordering, and array alias semantics. The functions are local and introduce no new capabilities, I/O, subprocesses, network calls, shared mutable state, or security boundary. |
| 10–11 | Runtime and Rust interfaces, CLI exit behavior, file formats, schemas, config, environment, and dependency graph are unchanged. No cross-repository changes are required. |
| 12–14 | Added behavior coverage, updated standard library documentation and changelog, and used repeatable timing observations independent of CI thresholds. |
| 15–16 | Preserved existing entrypoints and diagnostics for scripts and agents, including public Rust helpers. Repository-wide tooling, release packaging, and general dead-code removal were outside scope. |
| 17–18 | Reran the test matrix after selecting changes based on the paired measurements. |

## Remaining work

**P1.** The string lexer's ordinary-character loop recounts the accumulated segment
with `chars().count()` before checking whether the size diagnostic was already
reported. This is separate from the runtime builtin work. A single exploratory
`kujo check` process per case used 187.30, 222.48, and 282.17 CPU ms with the
same 256 KiB of valid ASCII literal content split into 512-, 4,096-, and
8,192-character literals respectively. Those measurements include the rest of
the compiler. The source establishes the repeated prefix scans directly. SignalBox review item
`sig_cf97e73c-df75-42dc-bd57-29efb60766aa` tracks incremental scalar accounting
with existing escape, interpolation, and diagnostic behavior.

## Verification

The selected baseline matrix passed 1,799 Rust tests. The final matrix passed
1,800, including the new builtin parity test. Both runs had nine ignored
tests. The baseline used three Cargo build jobs. The final run used two build
jobs and `RUST_TEST_THREADS=4`. An earlier concurrent run encountered eight
PDF test failures, including render deadlines. All eight passed in the final
run. Disk pressure led to cancelling the initial release build and reclaiming
generated release artifacts before the successful debug build.

```sh
cargo fmt --check
git diff --check
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 \
  cargo test --locked --no-fail-fast --lib --test vm_interpreter_parity_surfaces \
    --test interpreter_tests --test cli_contracts --test cli_json_contracts \
    --test diagnostics_golden --test docs_examples --test readme_contracts \
    --test schema_vector_hardening_contracts
target/debug/kujo test --runtime vm
target/debug/kujo test --runtime dual
python3 docs/audits/evidence/kujo-builtin-speed-2026-11-25.py BASELINE_KUJO CANDIDATE_KUJO
```

The baseline and final CLIs each passed 150 runnable fixtures in VM and dual
mode, with 11 inventory skips and zero dual-mode interpreter fallbacks.
The parity script embedded in the new Rust test also ran through both CLI
entrypoints. Baseline and final output matched byte for byte in each mode.
