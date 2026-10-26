# Imported native-default sharing — locally verified

Runtime commit `8b4111b` shares immutable native-default metadata in module
environments. Correctness, allocation and seven-pair local performance gates
passed. Machine-readable evidence is in `imported-native-defaults-2026-10-26.json`.
Clean-host, production-size, platform and competitive acceptance remain pending.

An initial prototype was rejected: a targeted native-loop test showed assignment
to a shared default creating a function-local binding instead of updating its
global owner. The committed correction includes shared defaults in lexical
membership checks. Fresh verification confirms both explicit and compact native
bindings update their global owner during actual native-loop execution.

This candidate follows the environment clone attribution at `e97f222`/`7d53f9e`.
The comparison baseline is runtime `0d0f365`, not the original performance baseline.
It targets repeated copying of native preload entries in module captures.

## Representation and invariants

After module evaluation, unchanged mutable canonical native preloads move into a
private `Arc<HashMap<String, String>>`. The table contains only immutable names;
it contains no shared mutable Values or writable scope maps. User globals, native
aliases, overridden builtins and immutable declarations remain explicit bindings.

Lexical lookup consults explicit scopes first, then native defaults. Assigning a
default creates an ordinary global override; local shadowing still wins. Override
writes participate in capture journals, while failed or rejected conditional
mutations publish no writes. Global snapshots and swaps retain their associated
defaults. Closure capture selects exactly the same visible names as before instead
of granting every closure ambient access to the full native registry.

`Environment.scopes` keeps its existing `Vec<HashMap<String, Value>>` type and
native mutable-pointer targets. It now enumerates explicit bindings for compacted
module captures; `Environment::get` includes their implicit native defaults. This
representation distinction is documented on the field. No general-purpose map
copy-on-write storage or native pointer replacement is introduced.

## Verification

The corrected candidate has passed the normal release build, `cargo check`, and
all **1,114 library tests** (zero failed, eight ignored), **204 integration tests**
(zero failed, one ignored), and all **150 test-size checksums** across VM, interpreter and JIT.
The actual native-loop proof also passed: both explicit and compact module
environments updated the global owner, with two recorded native-loop executions
and zero whole-function native entries. All 84 corrected priority comparisons
and four separate resource observations passed their checksums. The separately prepared indexed-assignment regressions target the next
compiler fix and are excluded from this candidate gate.

The rejected initial prototype passed 203 integration/source-unit tests (zero
failed, one ignored) and all 150 test-size checksums. It also completed all 56
read-only, 52 write-heavy, 84 priority and four resource observations. Those
results do not certify the corrected source. The prototype's NeuralNet JIT median
elapsed ratio was 1.07181, above the 5% review threshold; thermal contention was
observed, but did not clear that result. Fresh corrected-candidate NeuralNet JIT
pairs have a median ratio of 0.9952: the prior median slowdown did not repeat.
The conditional 11-pair recheck was therefore not triggered; it was not run.

Imported builtin-override counters recorded zero native entries, so those tests
verify fallback behavior. The separate forced-loop regression proved native
execution and correct global ownership in both explicit and compact environments.

Four new environment unit tests cover shared metadata versus independent overrides,
permissions, write journals, rejected writes, alias retention, root swaps and exact
closure-visible names. An allocation regression loads a real module and rejects
per-snapshot copying of the native preload map. A language fixture covers imported
recursion, helper calls, aliases, immutable bindings, callable overrides, closures,
callbacks, returned snapshots and caller isolation on all three runtimes.

## Measurements and acceptance

For 64 imported read-only calls, measured loop allocation requests fall from
61,977 to 3,353; requested bytes fall from 11,334,789 to 291,973. Array widths
128 and 2,048 have the same counts in both VM and JIT-enabled probe modes.
These probes run the imported fallback path.

Seven alternating pairs of 4,096-call local diagnostic runs give candidate/baseline
median elapsed ratios of 0.107 (VM), 0.089 (interpreter), and 0.104 (JIT-enabled).
The corrected write-heavy control gives 0.103, 0.091, and 0.104 respectively. These are narrow
helper-setup observations, not whole-language or competitive speedups.

All priority medians remain within the 5% slowdown review threshold. Individual
pairs vary on this loaded laptop; these are seven-pair diagnostic observations,
not clean-host speed or equivalence guarantees.

| Workload | VM median ratio (range) | JIT median ratio (range) |
| --- | --- | --- |
| QuickSort | 0.9974 (0.9797–1.0698) | 1.0104 (0.9312–1.0360) |
| MergeSort | 1.0048 (0.9619–1.0454) | 1.0135 (0.9613–1.0578) |
| NeuralNet | 1.0005 (0.9534–1.0793) | 0.9952 (0.9584–1.1607) |

Separate NeuralNet peak-RSS observations were 17,891,328→18,362,368 bytes (VM)
and 35,065,856→35,561,472 bytes (JIT). These single resource runs are excluded
from timing ratios and do not establish retained-heap or clean-host memory behavior.

Corrected artifacts and frozen binaries live in
`.audit-evidence/import-native-defaults-final-2026-10-26/`; the rejected prototype
remains in `.audit-evidence/import-native-defaults-2026-10-25/`. Compilation precedes all
accepted timing samples. No clean-host, production-size, platform, or competitive
acceptance is claimed. User-defined environment binding costs remain a separate
scaling concern; this change targets native preload copying specifically.

## Remaining user-binding scaling

A separate allocation-only control ran 64 imported read or scalar-write calls
with 0, 32, 256 and 2,048 unused mutable scalar module globals. All 32 baseline
and candidate VM/JIT-enabled observations completed with matching checksums.
Elapsed timings were not collected; these counts overlapped correctness/build
work and cannot support timing claims. Raw sources, identities and observations
are retained in the final audit directory under `user-bindings/observations.json`.

| Extra module globals | Corrected VM read requests | Corrected VM requested bytes |
| --- | ---: | ---: |
| 0 | 3,551 | 293,478 |
| 32 | 5,599 | 805,862 |
| 256 | 19,935 | 5,561,062 |
| 2,048 | 134,623 | 43,704,550 |

Requests increase by exactly 64 per unused binding: one copied name per binding
per call. Write controls show the same slope. Native-default sharing removes
native preload copying; it does not eliminate copying of arbitrary user globals.
Section 3 therefore remains open for value-environment materialization. Any next
representation must preserve shared module state, closure snapshots and native
lookup/pointer ownership; replacing maps broadly without those gates is unsafe.
