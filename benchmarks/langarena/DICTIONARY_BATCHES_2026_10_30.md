# Bounded root dictionary collection batches

Runtime `8ab7c6a` batches eligible multi-key root dictionary updates with arrays,
dictionaries, structs and mixed primitive values. Existing compact scalar and
same-key paths remain. Keys borrow existing owner handles when available;
snapshots copy handles, publication moves values, and owner identity is checked.

The private table starts small and grows only for distinct keys, up to the proven
write limit. Rewriting two keys 8,192 times does not reserve 16,384 slots. Tests
cover full-table replacement, bounded rejection, promotion, snapshots, insertion,
callbacks, error publication and owner replacement. Struct payloads retain the
lexical environment because a later indexed read can invoke a struct operator.
Conservative fallback remains for callbacks, spreads and unproven expressions.

Requested bytes per added call at dictionary width 2,048, with an initial retained
snapshot, compared with accepted `b1bf786`:

| Payload form | Before | After |
| --- | ---: | ---: |
| arrays-pair | 632722.0 | 6221.0 |
| dicts-three | 633870.0 | 7344.0 |
| structs-three | 633883.0 | 7337.0 |
| scalar-first | 632954.0 | 6428.0 |
| collection-first | 632954.0 | 6428.0 |
| same-key-mixed | 6152.0 | 6152.0 |

VM and JIT agree for these rows. The target fixed-write owner-width penalty is
removed across widths 8 and 2,048. These are allocation results, not elapsed-time
speedups or Goose/Rust comparisons. The same-key control remains part of the gate.

Verified gates:

- 1288 library tests passed; eight ignored. Exact prior test
  membership and added regressions are verified against the frozen source.
- 422 broader integration tests passed; one ignored. The portable dictionary
  contract runs in VM, interpreter and JIT, including lexical operator callbacks.
- All 150 test-size checksum cases passed: all 50 benchmarks in each runtime.
- 3,624 allocation pairs, 5,436 semantic runs and 18 boundary runs cover the
  target, dense/insertion/repeated-key controls, earlier row/selector fixes and
  historical ownership paths, nested dictionary payloads and capture count,
  long-name and scope-budget controls. Total allocation and slope regressions are rejected.
- The verifier rejects an unchanged baseline as a negative control.
- 252 alternating RSS pairs across 126 groups were reviewed explicitly.
  Fifteen additional alternating pairs rechecked the five largest positive means.
  Two follow-up means remain positive (about 95,573 and 62,805 bytes); a dense
  JIT pair rose 4,268,032 bytes while another fell 4,333,568 bytes. Both are
  retained and explicitly reviewed; no general RSS reduction is claimed.
  90 groups have positive means. The largest is 303104.0 bytes (2.420%).
- Formatting, type checks, source identities, storage layouts and diff checks pass.

Every RSS group retains its observations and review disposition. Loaded-host
process memory is not retained-heap attribution, leak freedom, clean-host
acceptance or a general memory-reduction guarantee. Any additional memory
follow-up used for acceptance must remain with the evidence.

The rejected first candidate passed 1,281 library tests but failed a new assertion
that incorrectly expected sparse eligibility for a local array declaration.
Conservative rejection was correct; the test now checks that rejection and that
capture skipping is disabled. V2 added the small initial table and distinct-key
growth guard, but the allocation gate rejected 16 cold/fresh struct rows at
three extra requests and 54 bytes. V3 stores short proof names inline, with
length-prefixed UTF-8 and an owned heap fallback. The representation retains
the prior vector size/alignment; tests cover empty names, Unicode, embedded
NUL, boundaries and cloning. A component allocation test reproduces the old
three-request/54-byte cost and removes it without increasing long-name costs.
V3 passed semantic checks but failed 116 historical allocation controls. Its
shared frame limit overreserved small array batches, and some rejected proofs
allocated temporary local-name sets before encountering unsupported syntax.
The final candidate preserves the tighter proven array capacity and checks syntax
before collecting names, using the same visitor for both passes. The isolated
analyzer comparison reduces rejected-proof setup from 324 to zero allocated
bytes, while the supported control stays at 240 bytes. All 116 prior regressions
are replayed early and remain included in the full historical gate; they are not
counted twice in the totals above. The final runtime gates independently verify
the actual workload result.


[Complete evidence](dictionary-batches-2026-10-30.json) preserves sources,
identities, raw outputs, independent verifiers, memory review and rejected-v1/v2/v3
provenance. The [nested dictionary payload baseline](NESTED_DICTIONARY_PAYLOAD_BASELINE_2026_10_30.md)
is rechecked in the acceptance gate and still requires a separate fix. Remaining capture/fallback and
representation review keep sections 3 and 4 open. Production, platform, compiled
execution and clean-host competitive acceptance remain separate. No Kostya contact.
