# Loop-local physical identity — candidate, not accepted

Baseline runtime: corrected accepted `5fe52fc5`; repository evidence head
`7a1347fe`. This is a correctness investigation following the rejected
[demand-admission experiment](DEMAND_ADMISSION_CANDIDATE_2026_11_03.md), not a
third admission-policy revision. Eager function admission remains unchanged.

## Reproduction and mechanism

Four small programs pass VM, interpreter and ordinary JIT on the frozen accepted
CLI, but fail with diagnostic `DISABLE_FUNCTION_JIT=1`:

| Shape | Expected | Baseline loop-only result |
| --- | --- | --- |
| Skipped earlier `i`, then a 300-trip local loop | `[300, 300]` | `null < int` error |
| One short for-loop followed by a hot for-loop | `44855` | `4955` |
| Mutually exclusive for-loop branches | `44850` | normalized-iterable error |
| Two hot for-loops, called twice | `89700`, `89700` | `49800`, `44850` |

The compiler assigns separate physical slots to lexical bindings and iterator
temporaries. Distinct slots can have equal source names. The loop translator
currently allocates SSA storage by those names, and initialization/persistence
finds the first matching slot. That merges distinct bindings and can restore the
wrong value or kind. The candidate must demonstrate actual correction through
compiled execution; source reasoning alone is not an acceptance gate.

The candidate uses physical-slot storage keys in loop regions. Original source
names remain available for runtime/diagnostic hashes. Integer-dictionary helper
slots retain their physical-slot path without an SSA alias. Whole-function
translation, admission thresholds, benchmark inputs and language semantics stay
unchanged. Storage keys are compiler metadata, not user-visible binding names.

An additional readonly control reports exactly
`Cannot mutate immutable let binding: fixed` in all four baseline modes. Repeat
it on the candidate to ensure storage keys do not leak into diagnostics.

## Verification gates

- [x] Four reduced failures reproduced on frozen accepted baseline; all 16 outcomes retained.
- [x] Original Words/SHA256 failures and extracted controls preserved in rejection archive.
- [x] Candidate source and cross-runtime regression test prepared.
- [x] Formatting and whitespace checks pass.
- [x] Full library suite: 1,316 passed, zero failed, eight existing ignores.
- [x] New regression: exact results plus actual native entry and completion; completed loop-condition integration executable passes all 15 tests.
- [x] All eleven selected integration suites complete: 69 passed, zero failed or ignored; exact test membership and log hash verified.
- [x] Original full test-size Words/SHA256 workloads and extracted failures pass in all four modes, including repeated Words calls (8/8 extracted controls).
- [x] Diagnostic names, fallback and readonly errors pass the loop-condition suite; the full library run also passes the explicit specialized dense-slot native-write/snapshot/readonly test.
- [x] All 50 diagnostic loop-only outcomes compared: 48 passes versus baseline 46; Words/SHA256 fixed, same two sorting deadlines retained as incomplete.
- [x] All 150 unchanged standard test-size benchmark checksums; independent verification checks all outputs, configurations and 171 source/config identities.
- [x] All 120 admission controls preserve exact compilation thresholds and native-entry counts; independently verified against unchanged policy expectations.
- [x] Bounded compilation-allocation comparison reviewed: 48 cells; median byte deltas below 0.1%, request-count increase at most about 0.7%; one production Words process peak recorded separately. No speed or retained-memory claim.
- [ ] Applicable production/platform checks and accept/reject review, commit/push.

Local evidence lives under
`.audit-evidence/native-performance-campaign-2026-11-03/loop-slot-isolation/`.
`state.json` retains source hashes and the active library handle. The baseline
all-50 loop-only collector records exact argv, source/binary hashes, stdout,
stderr, exit/signal/timeout and measurement receipts, including failures. Runs
may overlap compilation; their elapsed time is not a performance comparison.

The initial library build was deliberately cancelled after the formatting check
requested whitespace changes. Its exit130 is not a test result. The formatted
build is a separate recorded invocation. No candidate acceptance or speedup is
claimed while verification is pending.

## Wider baseline loop-only sweep

All 50 unchanged test-size inputs now have verified diagnostic outcomes: 46
checksum passes, Words wrong checksum, SHA256 iterator error, and QuickSort/
MergeSort exceeding the explicit 120-second process deadline (SIGTERM). This
diagnostic mode differs from normal JIT, and runs overlapped compilation; the
timeouts do not establish a source-caused performance regression or a speed ratio.

The original collector stopped after 23 recorded rows when an empty QuickSort
metrics file raised a JSON parse error. Its incomplete QuickSort row is not used.
A corrected collector preserved missing/partial metrics and recorded cases23–49
in a separate directory. Exact ordered union coverage, configuration, invocation,
input hashes, raw stdout/stderr, process outcomes and available metrics are
verified in `baseline-all50-verified.json`. Ruby's nil success status on a signal
is treated as a nonpass only with a recorded signal, never as a successful run.
Candidate loop-only results remain pending.

## Integration build resource limit

The initial eleven-suite release integration build encountered an explicit
`No space left on device` compiler error in `jit_callback_assignment`; the
`assignment_writeback` compiler process also exited 101. These are build failures,
not runtime regression outcomes. Preserve the failed log and wait for its remaining
compiler process before retrying with one build worker. No integration acceptance
is inferred from the library pass.

A separate four-mode reduced collector now verifies all five fixtures, including
the readonly diagnostic. The accepted baseline passes 16/20 controls: the four
known loop-only failures remain, while readonly behavior passes in every mode.
Loop-only success additionally requires native entry and, except for the intended
readonly exception, native completion. Sources, binary hashes, invocations and
raw outputs are retained in `baseline-reduced-native/report.json`. Candidate
results must use the same collector; these build-overlapping runs are not timings.

The frozen candidate CLI (`b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`)
now passes all 20 reduced controls, with native entry/completion required as above.
Exact matched coverage against the baseline is verified. The extracted original
Words repeated-call and SHA256 cases also pass in VM, interpreter, ordinary JIT
and loop-only JIT (8/8); the full workload sweep remains separate. The initial
build terminated with exit101 after finishing its CLI compiler; the eleven-suite
integration retry uses one build worker and remains pending. These results do
not yet accept the patch or establish a performance improvement. The completed
loop-condition test executable subsequently passed all 15 tests, including both
new regressions; its hash and the exact candidate CLI hash are retained in
`loop-conditions-direct-identity.json`. The subsequent serial integration invocation completed successfully: all eleven
suites passed 69 tests, with zero failures or ignores. `integration-verified.json`
retains exact membership and the complete log hash. Earlier partial results
below are historical and are superseded by that complete invocation.

While the resource-failed test builds finish serially, direct execution of the
eight other completed integration binaries passes 50 tests (no ignores), with
exact test membership, log hashes, executable hashes and the candidate CLI hash
retained in `completed-integrations-verified.json`. The newly completed
`assignment_writeback` executable also passes both VM/interpreter tests. Together
with the 15 loop-condition tests, 67 tests across ten of the eleven selected
suites pass. The final serial Cargo outcome subsequently verified all eleven suites; the
partial direct runs remain supporting evidence rather than additional unique tests.

Specialized-slot evidence is the passing library test
`jit::tests::guarded_loop_dense_writes_inline_without_rust_vec_layout_assumptions`.
It compiles an explicitly selected physical dense slot, executes without the name
map required by the generic write fallback, verifies unchanged unique storage,
rejects a shared snapshot view and checks the source-name readonly error. This
supports the specialized-slot gate alongside the new shadowing/iteration tests;
it does not claim exhaustive coverage of every specialization combination.

`compile_allocations.rs` prepares 24 bounded compilation-allocation cells over
0/16/128/512 unrelated locals and short/long names. It excludes parsing and JIT
construction, counts requested allocation bytes during compilation, and makes no
retained-memory or timing claim. The serial integration build is complete; the allocation probe build is now
pending separately. The current library is
identified from Cargo's actual `--extern kujo` argument and hashed separately;
`target/release/libkujo.rlib` is stale and must not be used for this candidate.

Standard VM, interpreter and ordinary JIT each pass all 50 unchanged test-size
workloads (150/150). Independent verification checks exact case order,
configuration, full output/checksum lines, exit status, binary identity and all
171 source/config identities. `all50-independent-verification.json` retains the
report hashes. This includes full test-size Words and SHA256 in all three modes,
distinct from the extracted controls and loop-only sweep. The 120 admission
controls retain the prior policy's exact
compile/native-entry counts across cold, threshold-boundary and hot cases.
`candidate-admission/verified.json` records independent verification of all raw
outputs, generated controls, invocations, measurement receipts and identities.
Its instrumented wall times overlap compilation and are not speed evidence.

The candidate loop-only sweep is complete and independently verified against
raw receipts: 48 passes, with QuickSort and MergeSort both exceeding the same
120-second deadline and receiving SIGTERM as on the baseline. Words and SHA256
now match their unchanged expected checksums. There are no newly failing rows;
the two timeouts remain incomplete, not passes or evidence of a speed change.
`candidate-all50-loop-verified.json` records exact ordered coverage, arguments,
configuration, source/binary hashes, raw outputs and measurement receipts.

The first probe build rejected the frozen library filename because Rust requires
a `lib` prefix; a hash-verified symlink resolved that naming issue. A second build
with LTO disabled failed because Apple LLVM 17 could not parse the Rust LLVM 22
bitcode in the frozen dependencies. Both failures are preserved as probe-build
evidence, not runtime failures. The current sequential baseline/candidate probe
build uses `opt-level=3` and Rust `lto=thin`, consistent with the earlier diagnostic
harness. Its state is `compile-allocation-v3-state.json`; no allocation result or
acceptance is claimed until both builds, executions and independent review finish.

## Local verification disposition

The fourth allocation-probe build uses a regular hard-linked `libkujo.rlib`
filename rather than a symlink resolving to `accepted-libkujo.rlib`. Both frozen
library hashes remain unchanged. Rust thin LTO then builds and runs both probes
successfully; earlier failed builds remain archived. The baseline's JIT,
compiler and bytecode sources are identical between `782c712d` and corrected
call baseline `5fe52fc5`, checked with Git. The probe invokes compilation only.

Independent verification covers 24 cells per library: four local counts, two
name widths and three repeats, with exact slot counts and build/library/binary
identities. Across eight groups, candidate median requested-byte changes range
from -0.0665% to +0.0990%; request-count increases range from +0.0320% to +0.7045%.
At 512 short-name locals, requested bytes are 231,168,869 versus 231,221,329.
These large totals are transient allocation traffic, not peak/live heap. Hash
iteration and compiler paths vary across repeats; small differences are not a
speed benefit or a universal regression guarantee. `compile-allocation-verified.json`
retains all 48 raw rows and grouped comparisons.

The unchanged production Words workload also passes in diagnostic loop-only JIT:
50,000 words, word length 4, 360 measured iterations, checksum `1998586832`.
Its metrics record 90,321 native boundaries, zero native function entries and
59,305,984 bytes peak RSS. The raw report/configuration, metrics hash and frozen
candidate identity are independently verified in
`production-words-loop-only-verified.json`. This is one production cell, not
all-50 production coverage; build overlap prevents a speed claim.

Local evidence supports committing the correctness patch for exact-revision
platform CI. Final platform acceptance remains pending, as do the broader
campaign's production, clean-host, architecture and performance requirements.
No admission-policy change or new performance optimization is accepted here.
`loop-local-identity-2026-11-03.json.gz` preserves the portable local evidence.
# Linux gate disposition

Run `41588426114` completed on exact runtime commit
`537406d8afb9e2f400111cf6ac8d0cfe3a7412a2`. The optimized loop job passed
1,356 tests with zero failures and eight existing ignores. Its recorded
`src/jit.rs` and `tests/jit_loop_condition_calls.rs` SHA-256 hashes match the
current accepted sources. Linux used rustc 1.99.1, x86_64-unknown-linux-gnu.
Formatting, Clippy, MSRV, VM/interpreter parity and minimal release smoke passed.

The full release gate failed in `generated_artifact_freshness_contract`: the
unsafe and TODO inventories retained old source line numbers. Regenerating both
with their strict generators changes only row line numbers and generation dates:
299 unsafe rows and 25 TODO rows retain their contents/classifications. A second
generation matches all four files exactly; formatting and diff checks pass.
No freshness assertion, test or classification was relaxed. The complete release
gate must still be rerun on the corrected commit; this is not a full release pass.

`loop-linux-gates-2026-11-03.json.gz` preserves the source/toolchain receipt,
optimized test log and inventory verification. The separate memory-lifetime
candidate is not included in this Linux run.
