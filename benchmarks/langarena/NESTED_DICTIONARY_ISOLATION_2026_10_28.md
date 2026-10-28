# Nested dictionary leaf isolation — 2026-10-28

The accepted runtime `924e949` still copies wide dictionary leaves during imported
helper updates. This holds for dictionary, struct and array owners, and for
numeric and string leaf keys. The latest inline-pair fix concerns direct captured
dictionaries; it does not cover these nested paths.

[Machine evidence](nested-dictionary-isolation-2026-10-28.json) records 144 allocation
measurements and 216 passing semantic cases, using the matching release binary
and allocation probe from the accepted dictionary-pair milestone. Independent
verification checks exact coverage, fixture/artifact identities and derived deltas.

Requested bytes per added helper call, VM, initial-only snapshot:

| Owner / leaf key | Width 2 | Width 128 | Width 2,048 |
| --- | ---: | ---: | ---: |
| Dictionary / numeric | 6,199 | 44,755 | 632,275 |
| Dictionary / string | 6,334 | 44,890 | 632,410 |
| Struct / numeric | 6,204 | 44,760 | 632,280 |
| Struct / string | 6,339 | 44,895 | 632,415 |
| Array / numeric | 5,570 | 44,126 | 631,646 |
| Array / string | 5,705 | 44,261 | 631,781 |

The runner also measures JIT allocation traffic and verifies VM, interpreter and
JIT results. It compares 16/32 calls at each width, with initial-only and fresh
per-call snapshots kept separate. Construction aliases are removed before calling
helpers. Saved values and an untouched neighboring key are asserted after mutation.
Allocation traffic is not elapsed time or retained heap; these are local diagnostics.

Source inspection explains the eligibility gap: `NestedPublication::admission`
and `RowKey::row` admit only array leaves. Nested dictionary updates therefore
cannot use the existing row journal. This is consistent with the measured scaling;
it is not proof that every allocation originates at that one site.

The next fix must retain dictionary scalar writes privately without enlarging
ordinary assignment-path or capture-frame storage. Acceptance requires:

- All three owner types and numeric/string keys; canonical key aliases and errors.
- Exact integer/float/Boolean/null values, repeated writes and ordinary fallback
  for other values, missing keys, larger batches and deeper paths.
- Original/fresh snapshots, shared leaves, callbacks, reentry, root replacement,
  completed writes before errors and established object/index evaluation order.
- Flat eligible leaf-width allocation growth, with no regression to existing
  array-row, direct dictionary, escape or fallback controls.
- Component/layout tests, regression tests across all three runtimes, matching
  release allocation evidence and the unchanged 150 test-size checksums.

Sections 3 and 4 remain open. No runtime source changes are included in this
isolation. Production, platform and clean-host comparison gates remain pending.
