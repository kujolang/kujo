# Compound indexed dictionary batches

Runtime `7724558` batches eligible indexed arithmetic in captured dictionaries.
A separate proof admits literal or identifier selectors. Binary and unary
operator callbacks temporarily lose the caller's publication permissions;
ordinary reads and calls publish writes when required. Permissions are restored
on return, including errors. Callback snapshots and selectors that replace the
root retain their existing semantics.

Requested bytes per added call at dictionary width 2,048 (VM and JIT agree):

| Dependency | Baseline `30e1511` | After |
| --- | ---: | ---: |
| Parameter | 633,044 | 7,062 |
| Captured read | 633,043 | 7,061 |
| Captured write | 633,051 | 7,069 |
| Captured increment and return | 633,051 | 7,069 |

All eight target groups have flat width scaling across 128 and 2,048 entries.
These are allocation slopes, not elapsed-time speedups.

Verified acceptance:

- 1,263 library passes; eight existing ignored tests.
- 414 integration passes; one existing ignored test.
- All 150 test-size checksums across VM, interpreter and JIT.
- 656 paired allocation comparisons and 984 associated semantic checks, with
  no allocation-total or added-call slope regressions.
- 15 additional historical contracts and 18 portable callback regression cases.
- 96 paired peak-memory observations across 48 reviewed groups.
- Exact source/build identities, matching baseline dependency flags, complete
  test-name coverage, formatting and diff checks.

The [first candidate](COMPOUND_PUBLICATION_REJECTION_2026_10_29.md) forced
publication before every operator callback and increased copying. The accepted
revision has zero allocation deltas in all 16 callback-body controls and passes
the original operator controls without their earlier 43-byte increase.

All 12 large-dictionary groups retaining only the original snapshot reduced mean
peak RSS by 20.33–20.81%, roughly 9.9–10.2 MB. Small and retained-snapshot results
had mixed signs. The largest small mean increase was 131,072 bytes/1.733%, positive
in both observations. The largest large retained mean increase was 628,736
bytes/0.905%, with paired changes of -606,208 and +1,863,680 bytes. The retained
write-only JIT case increased 448,512 bytes/0.644%, positive in both observations.
These increases are recorded tradeoffs, not dismissed as noise. Two local samples
do not establish memory neutrality, retained-heap bounds or leak freedom.

General compound selectors, capture traversal/materialization, unsupported
collection paths including scalar-first journals, and representation review
remain open. Sections 3 and 4 are not complete. Production-size and clean-server
Goose/Rust comparisons are separate pending work.

Evidence: [acceptance, observations and audit sources](compound-indexed-batches-2026-10-30.json).
