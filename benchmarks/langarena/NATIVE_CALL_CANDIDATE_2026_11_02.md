# Guarded compiled-call experiment: rejected and removed

The tagged-transfer tactic is closed without an accepted runtime change. It
passed correctness and improved isolated call controls, but two matched-build
workload batches did not establish convincing repeatable target gains. This is
rejection for insufficient benefit, not proof of a causal performance regression.
The broader known-callee/call-frame work remains open. Do not repeat this tactic
without new cost attribution; the next bounded campaign area is JIT admission
and worker compilation economics.

The candidate runtime, measurement schema/test, and changelog edits are removed.
All **1,392 frozen baseline source/build/test/vendor identities** match again;
`cargo fmt --check` passes. The accepted invalid-call recovery fixes remain.
**Local `target/release/kujo` still contains the rejected candidate.** Use the
frozen corrected baseline below; rebuild before treating target output as current
source. No release or maintainer contact occurred.

## Workload decision

Two separate local batches each verified 126 samples: seven rotating rounds of
unchanged QuickSort, MergeSort and NeuralNet, baseline/candidate VM/JIT plus
frozen Rust and Goose. Exact configuration/checksums, successful exits,
nontruncated output, binary/input identity, timers and RSS receipts were checked.

Paired median baseline/candidate speedup (above 1 favors the candidate):

| Workload | A VM | B VM | A JIT | B JIT |
| --- | ---: | ---: | ---: | ---: |
| QuickSort | 0.967883x | 0.995268x | 1.003671x | 0.984977x |
| MergeSort | 0.947714x | 1.007315x | 1.018170x | 1.006024x |
| NeuralNet | 1.089584x | 0.980862x | 0.926673x | 1.009802x |

These are medians of seven paired ratios, not ratios of separate medians.
QuickSort does not show a repeated gain. MergeSort's small differences are not
convincing against visible local variation. NeuralNet's first negative signal
does not repeat; neither does a useful positive effect. VM controls also vary.
All samples and outliers remain. No correction multiplier, significance,
statistical-independence, clean-host or production claim is made. Competitor
loop outputs are rounded to milliseconds. Neither batch is an all-suite score.

## What the candidate changed and proved

The archived patch extracted the existing cached native invocation body and
added a guard on actual callable code identity, JIT enablement, arity and
closure/try eligibility before consuming arguments. Ordinary calls of arity
0–4 transferred tagged operands through an activation-local stack slot rather
than the VM operand stack; field receivers and larger arities kept the legacy
path. Dynamic rebinding, snapshots, error handling and all three diagnostic
environment observations were retained. This still used a Rust runtime helper,
argument Values and native roots; it was not direct generated callee invocation
or inlining. The optional transfer counter was experimental and is removed.

Evidence retained:

- 188 unique unit tests passed; seven existing performance microbenchmarks were
  ignored. Their binary/log hashes and runtime/vendor/Cargo sources were verified
  unchanged after the matched rebuild; they were not rerun or double-counted.
- Three call-contract tests passed against the matched frozen executable, as did
  164 further regression tests and 150 unchanged test-size checksum cases across
  VM, interpreter and JIT. This is 355 unique regression tests plus 150 checksums.
- Eighteen matched participation controls passed. Each additional 768 hot calls
  produced 768 guarded transfers/native entries, zero added generic dispatch and
  zero added compilation. Counts do not measure CPU share or root allocations.
- 476 matched call samples verified across 17 scalar/owned arity/width cells.
  JIT paired-median speedups ranged 1.038375x–1.149628x; 13/17 cells won all seven
  pairs. VM medians ranged 0.982325x–1.015706x. These microbenchmark gains do not
  override the unchanged-workload outcome.
- A separate [sort oracle](SORT_PORT_STATE_AUDIT_2026_11_02.md) validates full
  arrays and snapshots on the first candidate variant; its scope stays explicit.

## Provenance and reproduction

Corrected baseline runtime `5fe52fc5`, evidence base `6f53d3cc`; frozen CLI:
`.audit-evidence/native-performance-campaign-2026-11-02/call-semantic-v1/kujo`,
SHA-256 `bf5bd666143cabb2339a72a5635702a6c735639e25d12663396aea7a87abaed4`.
Matched candidate CLI SHA-256:
`a1f12c56c8944b3a0d722c501d678411226b31375bcc516af1eeb346cf4ec0a0`.

The first candidate built with `cargo build --release --bin kujo` differed from
the baseline Cargo test graph in 14 dependency fingerprints, including Cranelift.
Its timings cannot isolate the source change; its second timing batch was
explicitly stopped after 179 samples. Those artifacts remain qualified evidence.
The matched candidate used `cargo test --release --locked --test
jit_call_contracts -j 1`; actual CLI linkage was captured, all retained dependency
fingerprint fields matched, and Cargo exited 0. No builds overlapped matched timing.

`probes/compare-native-calls.rb` and `verify-native-calls.rb` reproduce/check call
controls. `compare-native-call-targets.rb` and `verify-native-call-targets.rb`
reproduce/check the workload batches. Pass frozen baseline, frozen candidate,
a new output directory and seven rounds to the comparison runners.

The [archive](native-call-candidate-2026-11-02.json.gz) contains **2,866
SHA-256-verified artifacts**: runtime patch/source, build/identity receipts,
correctness logs, original qualified evidence, matched timings, raw outputs,
verifiers and orchestration. Executables/libraries are excluded, with their
hashes retained. Restore the archived patch on `6f53d3cc` in an isolated checkout
and reproduce the matched build before treating a newly built binary as equivalent.
