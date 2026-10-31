# Sections 3 and 4: representation and acceptance review

Follow-up: `5a0632d` closes the reproduced greater-than-eight constant-site
reservation penalty described below. The [accepted report](CONSTANT_SITE_FIX_2026_10_31.md)
supersedes that specific residual; varying selectors and unsupported expressions
remain conservatively bounded. The other residual categories remain open.

Reviewed runtime `1ed1866`, documentation head `17cc1c2`. The current sequence of
reproduced fixes and its combined regression gates is accepted. This review is
complete; **the broader section 3/4 optimization requirements are only partially
proved**. Neither an unchecked historical item nor a passing representative
matrix establishes the performance of every capture or collection shape.

## Requirement dispositions

| Requirement | Evidence on the accepted candidate | Disposition |
| --- | --- | --- |
| Reduce reproduced capture setup, name-copy and call-frame costs while preserving snapshots and recursion | Capture-storage controls are included in the remaining-control refresh; historical/extended controls cover earlier ownership changes. Exact library membership includes host membership replacement, removal/reinsertion, pool admission and retained-name tests. | Reproduced eligible paths pass. Membership traversal and fallback materialization remain; do not mark the broad capture requirement fully complete. |
| Reduce module-width copying for insertion, nonscalar updates and nested owners | Combined historical, extended and remaining matrices pass against the accepted capture-storage runtime; earlier per-fix reports establish reductions against their own baselines. | Previously reproduced eligible shapes are fixed. General unsupported/deeper/escaping shapes are not universally covered. |
| Review dense arrays and fresh escaped snapshots | Current dense refresh: 48 allocation pairs and 72 candidate semantic runs. All measured slopes match the accepted baseline. | No new owner-width penalty is demonstrated for the tested once-snapshot dense updates. Fresh-snapshot copying and per-payload allocation remain explicit. |
| Remove repeated-position reservation waste | Sixteen target pairs and 24 semantic runs; width penalties of 9,840/167,280 bytes per call become zero. | Accepted in `1ed1866`; not a remaining implementation task. |
| Preserve value semantics, bounds, ordered errors and host-visible ownership | 1,307 library tests, 430 integration tests, portable VM/interpreter/JIT regressions, 18 host contracts and exact test membership checks. | Passed for the changed paths and recorded corpus. Eight library tests and one integration test remain ignored as recorded. |
| Review memory and regression tradeoffs | 3,832 strict allocation pairs, 72 reviewed boundary pairs, 5,856 candidate semantic observations, 122 JIT boundary follow-up pairs, 48 RSS pairs and 55 RSS follow-up pairs. | Scoped acceptance, with positive RSS observations retained. Not zero-regression or universal memory claims. |
| Verify all benchmark checksums on the final runtime | 50 test-size checksums in each of VM, interpreter and JIT. | 150/150 pass. Production-size, platform and competitive acceptance remain separate. |
| Evaluate alternate collection representation | Chunked-owner experiment includes public contiguous publication and is worse in all 27 matched cases. | That design remains rejected. It neither proves every alternative bad nor fixes fresh-snapshot copying. |

## Costs that remain in the implementation

1. **Capture membership traversal.** `Environment::scopes` is a public vector of
   hash maps. `capture_activation_with_pool` validates compact metadata with
   `matches_names`, and rebuilds membership after host edits. A same-size map
   replacement cannot safely be detected from length alone. The current API has
   no mutation-version hook covering arbitrary public map edits. Eliminating
   validation requires a separately justified host/API design; skipping the scan
   would break the tested contract. Scan cost remains even though repeated name
   allocation is removed in the measured eligible cases.

2. **Conservative capture and publication fallbacks.**
   `capture_prefix_snapshot` still refills scope/kind maps when transfer is not
   selected, and clones before snapshotting pending publication. The existence
   of this fallback does not prove that a remaining LangArena hot path reaches
   it. Earlier same-owner, foreign and split-owner cases are covered by accepted
   fixes. A new optimization requires a current reachable reproducer and cost
   attribution, rather than removing a necessary fallback on inspection alone.

3. **Fresh observable snapshots and payload allocations.**
   `ArrayBatchPublication::snapshot_value` and `publish` use `Arc::make_mut` on
   contiguous public arrays. Observably retained old values must survive; that
   semantic requirement does not require this particular copying algorithm.
   The measured fresh-snapshot model still contains `136 × owner_width` bytes
   per call; the once-snapshot model contains `667 × writes`. These are fits to
   four tested combinations, not universal complexity proofs. A different
   representation must account for publication, construction, traversal, host
   compatibility and nested-value behavior before claiming a saving.

4. **Bounded proof coverage.** The repeated-destination proof uses eight inline
   destinations, a 4,096-visit budget and depth limits. A ninth distinct
   destination or varying selector retains the write-count bound; unsupported
   expressions retain existing fallback. This is an intentional bounded
   implementation, not evidence that larger sparse patterns are optimal. The
   raw-endpoint and overflow checks must remain; normalization alone can confuse
   a varying negative-to-zero loop with a constant physical index.

None of these four residual costs has been erased by checking off this review.
The next implementation decision should come from a current measured workload
that reaches one of them. The evidence does not justify another broad refactor
or a claim that all possible collection costs have been eliminated.

## Verification provenance and practical limits

[Repeated-array acceptance](REPEATED_ARRAY_FIX_2026_10_31.md) and its compressed
108-artifact bundle contain the final combined controls, source hashes, exact
test membership, stdout checksums, verifiers, negative control and explicit
boundary/RSS reviews. [Capture-storage acceptance](CAPTURE_STORAGE_BUDGETS_2026_10_31.md)
records the earlier retained-memory tradeoff; the repeated-array candidate does
not remove it. [Shared capture metadata](SHARED_CAPTURE_METADATA_2026_10_28.md),
[dense-array review](DENSE_ARRAY_REVIEW_2026_10_30.md) and
[chunked-owner experiment](CHUNKED_OWNER_PROTOTYPE_2026_10_26.md) retain their
original scope and artifact identities.

The accompanying review manifest verifies that the current runtime/test files
match the accepted commit, checks every bundled artifact digest, and records the
specific source and tests used for this review. No new runtime changes or
benchmark runs were needed for this documentation review.

Loaded-laptop timing does not establish a speedup against Goose or Rust. Section
5 compiled-execution work, production-size all-50 runs, other supported platforms
and a clean comparison host remain separate requirements. No Kostya contact,
upstream submission or release publication was performed.
