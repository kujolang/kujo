# Demand admission: rejected for JIT correctness failures

The first candidate and sole revision are rejected. No admission-policy runtime
change is accepted. `src/vm.rs` and the candidate-specific execution-contract
change are restored to the accepted source. The demand-admission test remains in
the evidence archive, not the active suite. Independently baseline-verified worker
native/cancellation tests are retained in `ed067cb8`.

**Do not use `target/release/kujo` as the accepted runtime:** it still contains the
rejected candidate. The accepted frozen CLI is
`.audit-evidence/native-performance-campaign-2026-11-02/call-semantic-v1/kujo`
(SHA-256 `bf5bd666143cabb2339a72a5635702a6c735639e25d12663396aea7a87abaed4`).

## Hypothesis and bounded disposition

Revision 1 removed first-call whole-function compilation based on loop presence
from ordinary VM Call and the compact native callback dispatcher. Although its
library, admission and initial integration checks passed, map callbacks lost
native execution: that dispatcher has no hot-backedge loop admission.

Revision 2 retained eager compilation for compact callbacks and changed only
ordinary VM Call. It retained the 100-call function threshold and existing
backedge admission. Neither version solved straight-line late compilation or
per-task compilation reuse. Both were general policies, with no benchmark-name,
input, timer or algorithm changes.

Revision 2 passed 1,316 library tests (eight existing ignores), 25 integration
tests across five suites and 120 exact admission controls. The latter retain the
observed loop-plus-function double-compilation cost. Passing these gates did not
establish benchmark correctness or performance acceptance.

## Decisive full-workload gate

| V2 test-size sweep | Passed | Failed | Disposition |
| --- | ---: | ---: | --- |
| VM | 50 | 0 | Exact input/checksum coverage verified |
| JIT | 48 | 2 | Candidate rejected |
| Interpreter | — | — | Sweep stopped after JIT failure; not claimed |

- `Etc::Words`: checksum **738494915**, expected **580539813**.
- `Hash::SHA256`: runtime error **ForNext requires a normalized iterable**.

The JIT driver completed all 50 attempts and exited 4. Failures are retained,
not scored. No paired timing campaign was run on the incorrect candidate.
No production, clean-host or speedup claim follows.

## Isolation on the accepted baseline

Sixteen frozen-binary diagnostic executions cover both workloads, baseline and
candidate, in VM/interpreter/JIT/loop-only modes. Both binaries pass VM and
interpreter; accepted baseline passes ordinary JIT. Both fail loop-only JIT, and
the candidate fails ordinary JIT. `DISABLE_FUNCTION_JIT=1` is a diagnostic switch,
not an official benchmark configuration. This shows the policy exposed existing
loop-JIT correctness defects; it does not identify their precise source yet.

An extracted SHA256 program returns **1332896592** in baseline VM, interpreter
and ordinary JIT; baseline loop-only raises the same iterator error.

An extracted repeated Words program produces identical prepared-text checksum
**1728110206** in all four modes. VM/interpreter/ordinary JIT produce cumulative
results **193513271, 387026542, 580539813**. Loop-only produces
**193513271, 466004093, 738494915**. The first call is correct; later calls diverge.
The next correctness investigation must examine loop reuse/state restoration,
without assuming the precise cause or removing native coverage to hide it.

Simple byte/array summation, indexed byte summation and repeated identical-word
controls pass all four modes. Initial incomplete control files omitted the local
`vector` helper and failed in every mode; corrected controls and both sets of
outcomes are retained rather than treating fixture errors as runtime defects.

## Test changes and evidence

The first V2 integration invocation passed seven tests and failed the old test
that required eager whole-function diagnostics. Its trace showed successful
native hot-loop compilation/completion. A candidate-only assertion required both
compilation and completion; it is reverted with the rejected runtime policy.
A new detached-worker test initially captured a channel illegally; explicit
shared-store access fixed the fixture. The corrected task suite passes all four
tests against both the accepted baseline and V2, covering all runtimes. Parent
polling cannot satisfy the new worker-native check. Existing cancellation and
sentinel-pool coverage separately checks reclamation.

V2 CLI SHA-256:
`b46edc23651e390a81822256d2cd5f7cb7fdb81e8f9a37ce6045de43e66b2249`.
Its dependency fingerprint matches the baseline. Before testing, all 1,392
baseline identities were checked; only the intended VM and two test files differed.
Candidate snapshots preserve those identities after rejection.

Local evidence: `.audit-evidence/native-performance-campaign-2026-11-02/demand-admission-v1/`
and `demand-admission-v2/`. The portable archive
`demand-admission-rejected-2026-11-03.json.gz` retains source snapshots, patches,
raw outcomes, exact test membership, manifests, counters and diagnostic programs;
executables and libraries remain local. Every archived file hash is checked.

The admission tactic is closed. Next: isolate and fix the accepted loop-only
correctness defects under separate regression gates, then revisit compilation
architecture from new evidence. No third tactical admission revision is implied.
The broader eight-area campaign, all-50 port equivalence, production, memory,
platform and clean-host requirements remain open. No release or Kostya contact.
