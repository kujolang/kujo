# Indexed publication journal experiment — 2026-10-27

A bounded, inline write journal reduces complete update-and-publication allocation
in this standalone array model while preserving real pre-return public values.
This report records the standalone model, **not a measured language speedup**.
The subsequent [sparse runtime integration](INDEXED_PUBLICATION_RUNTIME_2026_10_27.md)
has its own allocation and correctness evidence. General captured-local and
broader module collection costs remain open.

The [source](probes/indexed_publication_journal.rs) uses the actual 136-byte Kujo
`Value` from the accepted `aee15a2` runtime, archived at evidence commit `0d54b47`.
The [evidence](indexed-publication-journal-2026-10-27.json) contains source, build
arguments, library/executable hashes, all rows, paired comparisons and the rejected
first experiment.

## Mechanism and modeled contracts

The public owner stays an ordinary `Arc<Vec<Value>>` throughout each invocation.
A private journal reads from that base plus its pending updates. At return it
drops its private reference before applying updates through `Arc::make_mut`.
An escaped public snapshot therefore still forces the required copy. If a host
replaced the owner during the invocation, publication uses the invocation's
original base plus its writes, matching the ordinary private-copy model.

The bounded version stores up to four distinct scalar writes inline. Repeated
writes replace the pending value at the same index. A fifth distinct write, a
non-scalar update, or an escaped private snapshot materializes the ordinary array
and continues using its existing copy-on-write behavior. A shared owner at entry
also uses that ordinary path. No pool budget increases or public markers are used.

The earlier growing `Vec` journal allocated extra storage. In its 72 comparisons,
30 increased requested bytes, including all during-call observer and private
snapshot cases. Some dense cases requested nearly three times the baseline bytes.
That design is rejected. The bounded version avoids its journal heap allocations.

## Results

The final matrix has **648 model runs**: three storage modes, lengths
128/2,048/32,768, 1/8/128/512 writes, three index patterns and six snapshot/publication
policies. Every run checks current values, an original retained snapshot and the
public pre-return values against a separate integer model. Additional controls
exercise scalar types, non-scalar fallback, buffer overflow and unchanged-owner
identity on a no-op.

Of the **216 bounded-journal comparisons**, 84 reduce requested bytes and 132
match the ordinary model. None increase allocation requests or requested bytes.

| Policy | Fewer bytes | Equal bytes | More bytes |
| --- | ---: | ---: | ---: |
| Original snapshot retained once | 27 | 9 | 0 |
| Fresh snapshot before every call | 0 | 36 | 0 |
| Observer retains public value during call | 0 | 36 | 0 |
| Private current value escapes during call | 0 | 36 | 0 |
| Host replaces public owner before publication | 27 | 9 | 0 |
| Publish only the simulated completed-write prefix | 30 | 6 | 0 |

For 32,768 elements, 128 writes per call and 16 calls:

| Index pattern, original snapshot retained | Flat requests / bytes | Bounded requests / bytes |
| --- | ---: | ---: |
| Spread across the array | 32 / 71,303,808 | 32 / 71,303,808 |
| Repeatedly update one index | 32 / 71,303,808 | 2 / 4,456,488 |
| Alternate between two indices | 32 / 71,303,808 | 2 / 4,456,488 |

The first call still separates the original snapshot. Later sparse calls can
reuse the current public allocation at publication. Dense calls fall back without
extra journal heap allocation. Setup of reference-checking buffers and simulated
external host replacement is outside measurement in both models; private updates,
escape materialization and publication are included.

The standalone active-state enum occupies 104 bytes versus `Value`'s 136 bytes.
This is a feasibility observation, not proof that an integrated representation
will preserve runtime frame, enum or stack sizes. Those must be checked directly.
There is no elapsed-time, peak-memory or generated-native coverage claim.

## Baseline language contracts

[Indexed-publication contracts](../../tests/indexed_publication_contracts.rs) pass
15 tests against the accepted `aee15a2` binary: five case groups in each of VM,
interpreter and JIT. They cover scalar and nested-value fallback, dense writes,
escaped arrays and captured locals, native operations, negative/float reads,
index expressions replacing the target, completed writes before bounds/type/
immutable/explicit errors, shadowing and nested snapshots.
[Baseline evidence](indexed-publication-contracts-2026-10-27.json) records the
source/harness/runtime identities and test output. These establish behavior
before integration; passing them does not prove the optimization is active.

## Required integration gates

### Compact storage candidate

At commit `b2b8448`, the test-only [storage candidate](../../src/interpreter/indexed_publication.rs)
narrows the four-slot model to two numeric/Boolean/null slots. Its 48-byte payload
fits the existing capture-write journal's largest payload on the local x64 build.
A test against the actual `CaptureWrites` type checks the candidate enum and its
enclosing `Option<(usize, ...)>`; the standalone layout probe reports 56 bytes for
both enums and 64 bytes for both enclosing fields. This does not establish layout
on other targets or prove a future integrated environment is unchanged.

The candidate takes an existing exactly-sized private map key without cloning it.
It rejects spare-capacity keys, unsupported values, oversized indices and a third
distinct update so the caller can retain ordinary behavior. Repeated writes to
the same two indices remain inline. Integer extremes, float bits (including NaN
payload and negative zero), Boolean and null round-trip unchanged. Publication
uses `Arc::make_mut`; retained observers and replaced owners keep their snapshots.

Storage tests include all 16,807 five-operation sequences drawn from three index
writes, nested-array fallback, public snapshot, private snapshot and public-owner
replacement. An independently allocated ordinary private-copy implementation is
the oracle. Each step checks the private reads and unchanged public owner, and
publication checks final contents plus every retained snapshot. Dedicated tests
also check atomic rejection, rollback, cloned pending state and pointer reuse.
[Storage evidence](indexed-publication-storage-2026-10-27.json) records the seven
passing tests, command and source hashes.

At that checkpoint the module compiled only under `cfg(test)`. The subsequent
runtime report is authoritative for integration status. These historical storage
checks alone do not prove runtime allocation reduction or replace runtime tests.

- [ ] Keep the journal private. Preserve public `Value::Array` owner values before
  return, and ordinary values at every host, native, output and snapshot boundary.
- [ ] Select a representation without enlarging ordinary values or call frames;
  measure cold setup, fallback, retained memory and recursion costs directly.
- [ ] Preserve index evaluation/replacement ordering, bounds/type errors, immutable
  bindings and completed writes before errors. The prefix policy above simulates
  partial completion; it does not execute Kujo exceptions.
- [ ] Normalize or safely carry pending writes through reads, alias creation,
  return, nested helpers, callbacks, reentry, generators, tasks and cancellation.
  Do not hold a public owner lock across user code or publish updates early.
- [ ] Extend evidence to actual dictionary and nested-row/struct paths rather than
  extrapolating the numeric-array model to those representations.
- [ ] Add regression tests in VM, interpreter and JIT; compare actual helper
  allocations with the frozen accepted runtime at identical paths.
- [ ] Re-run cold/append/module ownership gates, all 50 test-size checksums per
  runtime, formatting and relevant platform/feature checks for the integration.

The growing-journal and bounded-journal experiments are complete. Sparse runtime
integration is verified separately; broader collection integration, production-size
acceptance and clean-server comparisons remain open.
