# Capture-independent imported helpers — 2026-10-26

Runtime `2c342ef1817bc6931feb84cbfc039f3dc1f26f1f` is verified against the frozen `b34a0b8` baseline. This removes
repeated capture setup for eligible synchronous helpers. It is not an overall
Kujo speedup or a Goose/Rust ranking. Full production and clean-host acceptance
remain pending. [Machine-readable evidence](capture-independent-helpers-2026-10-26.json).

## Change and semantic boundary

Previously, a helper such as `func first(items) { return items[0] }` cloned its captured
module environment on every call, including unrelated bindings. The runtime now
caches a conservative analysis with the immutable function body. Every invocation
still checks the actual callable parameter names and relevant argument kinds;
embedding callers can construct different parameter lists around the same body.
One body lookup serves both the check and execution. The captured owner stays
alive, and eligible helpers use the existing isolated parameter/local scope and
unwind handling.

Unknown syntax, calls, methods, constructors, closures, suspension, typed
bindings and spreads retain ordinary setup. Operators and iteration require
primitive arguments: struct operators and generators can invoke lexical user
code implicitly. Catch blocks are excluded because they create named Error
structs. Scoped declarations cannot hide later global references. No benchmark
algorithms, inputs, checksums or workload names were changed.

## Measured allocation change

The table reports requests / requested bytes **per added call**, calculated from
matching 64- and 128-call executions. These are VM-entry measurements:

| Unrelated bindings | Array width | Baseline | Candidate |
| ---: | ---: | ---: | ---: |
| 0 | 128 | 57 / 4538 | 21 / 1241 |
| 0 | 2048 | 57 / 4538 | 21 / 1241 |
| 128 | 128 | 185 / 44220 | 21 / 1241 |
| 2048 | 128 | 2105 / 680788 | 21 / 1241 |

The read-helper cost is now independent of both unrelated binding count and
array width in these controls. JIT-entry results preserve a small difference:
each corresponding 64-to-128-call delta contains one additional request and
52 additional bytes versus VM (0.015625 requests / 0.8125 bytes per added call),
both before and after the change. The JSON retains every raw result.

Per-call deltas subtract startup and first-use analysis. Including import and
initialization, the 128-call VM case with 2,048 unrelated bindings fell from
302,150 requests / 97,136,612 bytes to 35,408 requests / 10,155,638 bytes.
Allocation traffic is neither retained memory nor elapsed speed.

## Costs that remain

Mutable-module writes are unchanged. VM per-added-call request/byte costs remain:

| Unrelated bindings | Array width | Requests / bytes |
| ---: | ---: | ---: |
| 0 | 128 | 47 / 21,728 |
| 0 | 2,048 | 47 / 282,848 |
| 128 | 128 | 175 / 61,410 |
| 2,048 | 128 | 2,095 / 697,978 |

The same small JIT delta described above remains. Helpers that require captured
bindings or unsupported operations still use the existing path. General imported
binding materialization and mutable-module array ownership therefore remain open.

Captured append controls at 512, 1,024, 2,048 and 4,096 elements retain the same
linear allocation slope in VM and JIT. The candidate adds a constant six requests
and 924 bytes at each size. That fixed cost is reported rather than hidden by the
per-added-call comparison. Cached proof metadata lives with its function body;
it does not retain an environment or collection snapshot.

## Verification

- Full default-feature debug library suite: **1,133 passed, zero failed, eight ignored**.
- Broad integration suite: **286 passed, zero failed, one ignored**. The three new
  VM/interpreter/JIT fixture tests are included in this count. The ignored test is
  `byte_concat_contracts::profile_byte_growth`, a manual profiling control.
- Five new proof/guard tests are included in the library count. They cover
  container projection, operator guards, public parameter-list changes, scope
  exit/global fallback and implicit callback paths.
- The new fixture also passed on the baseline. It covers module/caller shadowing,
  overloaded operators, COW snapshots, recursion, callbacks and error recovery.
- Ordered task and field/generator screens pass in all three runtimes. These
  overlap the broad suite and are not added to its distinct count.
- All **150 test-size checksums** pass: 50 each in VM, interpreter and JIT.
- 32 matching allocation controls and 48 read/write/snapshot semantic controls
  pass; eight append controls retain linear scaling.
- The committed allocation regression runner executes all 32 controls on both
  artifacts. Its flat-read-setup gate **fails on the baseline and passes on the
  candidate**. It compares exact integer deltas within each runtime, without
  hard-coding an absolute allocation count or claiming write costs are fixed.
- Formatting, whitespace and the frozen 812-source-file identity check pass.

The first allocation harness attempt was missing its imported fixture module.
The unchanged baseline module was restored, and execution resumed with the same
hash-verified compiled probe. That harness failure is retained in the evidence;
it was not a runtime fix or a discarded failing language test.

## Repeating the allocation gate

The existing [allocation probe](probes/captured-append-scaling/allocation_probe.rs)
accepts a runtime mode and a source path. Link it against the selected **normal
release library**, using that build's dependency/native-link flags; the evidence
JSON records the actual compiler argv and artifact hashes. Test-library feature
unification must not silently substitute a different library.

Run the committed regression driver with that executable and a new output folder:

```sh
ruby benchmarks/langarena/probes/imported-helper-allocations.rb /path/to/matching-probe /new/output/directory
```

The driver checks fixture hashes, keeps every result, and exits nonzero on runtime
failure or binding/width-dependent read setup. It uses the committed baseline
fixture/case reports and does not change the exercised Kujo code.

Local timings remain diagnostic because machine load varies. JIT entry does not
prove native execution of every helper. Later compiled-execution work, full-size
production runs, platform gates and clean-server comparisons remain unfinished.
No maintainer contact or upstream submission was made.
