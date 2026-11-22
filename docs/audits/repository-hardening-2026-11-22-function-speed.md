# Kujo function performance hardening

## Repository and baseline

`kujolang/kujo` on `main`, starting at
`4eb08be3ede0c4d3b1df3e4952c7ae3784f0e038`. Verified source ends at
`86d77c0d8cf36dca6cd3a3e2fa23a6b75001830c`. Kujo uses a Rust bytecode VM by
default, a tree-walking interpreter on request, and Cranelift for experimental
native compilation. The pass covers language function dispatch and the
`join` builtin.

## Findings

| Priority | Finding | Evidence | Resolution |
| --- | --- | --- | --- |
| P1 | JIT-enabled bytecode calls repeatedly inspected function instructions to choose dispatch and promotion paths. | `OpCode::Call` scanned for backward jumps even on inline-cache hits. Uncompiled calls scanned for two fused map instructions on every call. | Cache the instruction facts with immutable bytecode and check the environment override only when a call can trigger promotion. The existing code identity and mutation invalidation also apply to the new fact. |
| P1 | Closure frame setup copied both capture maps from an already owned callable. | `call_bytecode_function` rebuilt one map entry by entry and cloned the other. | Move both maps into the new frame. Escaped and recursive calls still share the original cells, and lexical self-binding retains its own references. |
| P2 | Interpreter arity checks copied names and parameters for valid calls. | Functions, methods, operators and generators constructed exact-arity descriptors ahead of validation. | Compare argument counts first and construct the existing diagnostic only on mismatch. Explicit `self` is excluded from method argument counts. |
| P1 | `join` duplicated existing strings. | The native handler copied string values and formatted other items into an owned string vector before constructing the result. | Reserve space for string items and separators, borrow the string bytes and format other items directly into the result. |
| P2 | An experimental one-pass `split` collector saved a temporary vector but increased CPU time. | The trial went from 845.87 ms to 950.12 ms median on 450 calls over 3,075 parts. | Reverted the experiment and used `split` as a control in the final comparison. |

The shared metadata test now covers fused instructions and cache invalidation.
The string regression test pins Unicode splitting on an empty delimiter,
leading and trailing empty parts, a mixed-value join, and an empty join.
Existing closure, arity and JIT tests cover the language call paths. The
optimizations preserve the public Rust helpers, Kujo signatures, diagnostics,
CLI contracts, schemas and configuration. The builds use the existing Cargo
features and locked dependencies.

## Measurements

Both binaries came from the locked default-feature test profile with debug
information and incremental builds disabled. The host was macOS 26.6.2 x86_64
with an Intel Core i7-9750H and Rust 1.96.0. Each workload ran in a fresh
process for one warm-up pair and five timed pairs, alternating binary order.
The harness verified the expected aggregate result and matching stderr on each
run. The table reports process user plus system CPU time, including startup and
compilation. Values are median milliseconds with the observed range in
parentheses.

| Workload | Baseline CPU ms | Final CPU ms | Median change |
| --- | ---: | ---: | ---: |
| Interpreter, 30,000 eight-argument calls | 1,505.63 (1,458.44–1,527.50) | 1,387.72 (1,358.02–1,473.93) | -7.8% |
| Interpreter, 30,000 explicit-self method calls | 1,280.76 (1,227.52–1,298.15) | 1,162.34 (1,106.87–1,183.91) | -9.2% |
| VM control, 90,000 eight-argument calls | 2,468.50 (2,388.11–2,595.72) | 2,540.96 (2,479.40–2,626.82) | +2.9% |
| VM, 65,000 calls to a closure with eight captures | 2,959.37 (2,862.34–3,049.83) | 2,302.23 (2,249.66–2,395.28) | -22.2% |
| JIT, 100,200 calls to a two-argument function with a guarded 96-statement branch | 5,319.98 (5,270.85–5,434.19) | 5,051.93 (5,015.56–5,107.81) | -5.0% |
| `join`, 450 calls over 3,072 string values | 590.58 (588.99–605.69) | 267.65 (264.08–268.60) | -54.7% |
| `join`, 600 calls over 2,304 mixed values | 713.66 (694.62–746.60) | 326.79 (323.48–438.33) | -54.2% |
| `split` control, 450 calls yielding 3,075 parts each | 745.13 (740.38–748.28) | 750.47 (746.34–756.04) | +0.7% |

The JIT workload also measured the elapsed time of 100,000 calls after 200
warm-up calls. Median elapsed time dropped from 2,994 ms (2,981–3,063) to
2,762 ms (2,720–2,801). A `try` block keeps the caller in the VM while the
eligible callee runs natively. The measurement counters verified one
compilation and 100,101 native function entries in each binary.

The plain-call control uses the same arithmetic as the eight-argument
interpreter workload and exercises default VM dispatch without captures.
Its CPU time ranges overlapped in the main run and in a separate five-pair
repeat. The repeat measured medians of 2,352.17 ms and 2,385.78 ms (+1.4%),
with ranges of 2,110.75–2,543.10 ms and 2,190.26–2,657.88 ms. These samples
leave the plain-call impact inconclusive.

The closure mutates one shared cell and reads seven others. The string-only
`join` workload repeats two UTF-8 strings and an empty string. The mixed
workload repeats a string, integer, float, two Booleans and null. The split
control uses a multi-character delimiter and includes empty parts. Its
aggregate count and both join byte lengths matched.


## Scope and verification

| Phases | Scope and evidence |
| --- | --- |
| 0–1 | Checked branch, starting revision, runtime features, prior hardening reports, language contracts and verification baselines. |
| 2–4 | Traced hot call dispatch, arity validation, frame capture ownership and native string allocations. Measured elapsed time and child CPU time. |
| 5–6 | Checked the documented compact CLI contracts and strict replay path for scope. Function dispatch changes run below the AI tool registration, replay/retry, and handoff layers. The benchmark verified output per process. |
| 7–9 | Preserved arity errors and validation, shared capture cells, JIT eligibility, bytecode mutation invalidation and native capability dispatch. These changes introduce no I/O, synchronized state or trust boundary. |
| 10–11 | Checked function and builtin contracts and Cargo features. No dependency or format changes. |
| 12–14 | Extended existing metadata and string tests, used current parity and JIT suites, and documented the changes in the changelog. Timings are observations, with no elapsed-time gate. |
| 15–16 | Kept the current runtime entrypoints and structured diagnostics. Removed redundant work within those paths. Agent tooling and dead-code removal were outside scope. |
| 17–18 | Implemented the four findings and reran the checks below. |

The clean baseline passed formatting and 1,821 Rust tests with
9 ignored. The final matrix passed 1,847 Rust tests with
9 ignored, including the new Unicode/mixed-value regression and
the extended bytecode metadata test. The Rust test runs used
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3`.

```sh
cargo fmt --check
git diff --check
cargo test --locked --lib --test vm_interpreter_parity_surfaces \
  --test interpreter_tests --test closure_capture_audit --test runtime_measurements \
  --test cli_contracts --test cli_json_contracts --test diagnostics_golden \
  --test docs_examples --test readme_contracts
cargo test --locked --no-fail-fast --lib --test vm_interpreter_parity_surfaces \
  --test interpreter_tests --test closure_capture_audit --test runtime_measurements \
  --test cli_contracts --test cli_json_contracts --test diagnostics_golden \
  --test docs_examples --test readme_contracts --test jit_loop_condition_calls \
  --test jit_callback_promotion --test jit_error_propagation --test fused_append_contracts
target/debug/kujo test --runtime vm
target/debug/kujo test --runtime dual
```

Both Kujo fixture modes passed all 150 runnable fixtures with 11 inventory
skips. Dual mode reported zero interpreter fallbacks.

The unrestricted test attempt was replaced by the listed matrix. The
`cargo build --release --locked` baseline was stopped when free disk
space fell to 1.6 GiB. The timing results describe the debug builds.
The raw timing pairs, probe, and test logs are under
`/tmp/kujo-function-speed.7E1p20/` on the measurement host.

The measurements characterize the selected workloads on one host. Other
platforms, broad ecosystem workflows, peak RSS and application throughput
were outside this pass. No cross-repository change is required.
