# Nested dictionary spill candidate rejected — 2026-10-28

The first overflow candidate is rejected and its runtime changes are reverted.
The accepted runtime remains `21a0907`. New regression tests and a maintained
five-write escape/fallback probe are retained for the next implementation.
[Evidence](nested-dictionary-spill-rejection-2026-10-28.json) preserves the rejected
patch, source/artifact identities, raw matrices and independent verification.

## What passed and what failed

The candidate promoted a full four-key inline journal to a sparse hash table on
the fifth distinct scalar key. It preserved assignment storage size and passed
1,208 library tests (eight existing ignores), new VM/interpreter/JIT tests across
three owners and batches through 257 writes, and 36 strengthened root-replacement
cases. The new runtime tests also pass on the accepted baseline. Fresh-snapshot
checks cover every updated key, rather than only the first two baseline keys.

Correctness was insufficient for acceptance. The five-write fallback matrix
completed all 240 allocation pairs and 360 semantic cases, but **80 allocation
pairs regressed**. Callback, snapshot-then-write, other-row, shared-row, nonscalar,
invalid-selector/index and overflow-error cases each add one allocation and
280 bytes per affected warmed helper call. This occurs at both widths 32 and
2,048 in VM/JIT modes with initial-only snapshots. Fresh-snapshot cases did not
produce those regressions.

The table allocation survives as overhead when the helper subsequently falls back
or exposes state. The gate was not weakened. A separate unchanged-baseline run
passes all 240 pairs with exact allocation equality and all 360 semantic cases.
Independent verification checks exact regression coverage and magnitude, fixture
and artifact hashes, baseline/candidate runtime outcomes and root-replacement
parity. The broader target and full acceptance pipeline were not run after this
failure; no speed improvement or accepted runtime fix is claimed.

## Next implementation direction

Avoid unconditional allocation at the overflow boundary. Investigate reusing the
existing `FunctionBodyRef::can_batch_dictionary_writes` proof and activation's
`allow_multi_key` policy for nested overflow, preserving the existing fallback
for uncertain or escaping helpers. This is a design direction, not a verified
fix. Check that proof coverage includes intended nested paths and reject unsafe
forms without changing their behavior or allocation costs. The current sparse-region
analyzer admits direct identifier indexing and rejects indexed arithmetic, so
simply passing its flag through will not optimize the measured nested compound
writes. Any extension needs runtime-type and callback/error-order coverage.

Dictionary semantics require associations and evaluation order, not an insertion
order guarantee (`docs/LANGUAGE_SPEC.md`, section 5.6). The earlier suggestion of
an ordered table was a possible design, not a new language contract. The next
candidate must preserve observable key enumeration and diagnostics under the
existing runtime contracts.

The [larger-batch acceptance gates](NESTED_DICTIONARY_WIDE_ISOLATION_2026_10_28.md)
remain open. Keep the new five-write fallback gate and strengthened root-replacement
probe in subsequent verification. Runtime fixes must pass both target improvement
and escape/fallback nonregression before being committed.
