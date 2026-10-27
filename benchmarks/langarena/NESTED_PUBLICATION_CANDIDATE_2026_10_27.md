# Nested publication storage candidate — 2026-10-27

This is a **test-only storage candidate**, not an enabled optimization. It follows
the [nested-row allocation isolation](NESTED_ROW_ATTRIBUTION_2026_10_27.md).

`src/interpreter/nested_publication.rs` reuses the two-element assignment path that
already exists when a nested write is evaluated. The first slot retains the
original root; the second holds the row selector, resolved index and scalar value.
The binding name moves from its existing storage. The journal contains only a
`String` and `Vec`, occupying 48 bytes locally. Tests compare the proposed capture
enum and its enclosing optional field against the actual current types, and check
that the assignment access enum does not grow.

Admission requires a struct/dictionary root shared only between its public owner
and private invocation, an otherwise unshared array row, a valid index and a
numeric/Boolean/null value. Repeated writes to the same location replace the
pending scalar. Other writes are rejected without changing the pending value;
the eventual caller must materialize and use the ordinary path.

The public owner retains real pre-return values. On publication, matching owner
identity permits releasing the private root before applying the write. Existing
snapshots still force copy-on-write. A replaced owner cannot become the private
base. String/integer dictionary selectors retain canonical spelling distinctions.

## Verification

All **1,172 library tests pass**, with eight existing manual profiles ignored.
Nine candidate tests cover:

- Reusing the existing name/path storage and a unique public row allocation.
- Late public snapshots, private snapshots and escaped row aliases.
- Private escape followed by another pending write, retaining both old snapshots.
- Atomic rejection of existing snapshots, shared rows, invalid paths and unsupported values.
- Owner replacement, including a changed struct name with the same field storage.
- Discarding pending writes without changing the original base.
- Canonical keys, integer limits, negative indices, bounds errors, Boolean/null
  values and preservation of floating-point negative zero.
- Unchanged path storage and proposed capture structure layouts on this target.

[Machine evidence](nested-publication-candidate-2026-10-27.json) records source
hashes, test arguments, test names and the log identity. Formatting and diff checks
also pass. The module, extra access variant and layout candidate are all gated by
`cfg(test)`; production execution remains unchanged.

## Required before enabling

This proves neither an allocation improvement nor a safe runtime integration.
Next work must connect admission after index evaluation and binding checks,
provide safe reads of pending scalar values, and materialize correctly on other
accesses, escapes, recursion, tasks and callbacks. Existing root-after-index and
completed-write-before-error contracts remain binding.

Measure complete mutation and publication, including combinations of public
observers and private escapes. Snapshot correctness alone does not prove that
copies were not duplicated. Review environment cloning so a publication journal
is not cloned only to be immediately filtered out. Test fallback after different
indices/rows, dense writes, multiple pending bindings, small rows and fresh
snapshots. Verify actual runtime structure sizes after integration, not just this
candidate enum.

Only then run matched allocation/scaling controls, VM/interpreter/JIT regressions
and all 150 test-size checksums. The nested-row item and broader work remain open
in the [full checklist](PERFORMANCE_CHECKLIST.md). No elapsed-speed, production-size
or clean-server claim follows from these unit tests.
