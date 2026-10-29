# General array batch copying: accepted-runtime baseline

Runtime e445f4a fixes eligible two-entry replacements. Counted loops that update
3, 8 or 64 distinct array entries still copy their owner once per helper call in
this matrix. Homogeneous array, alternating array/dictionary and struct payloads
all retain an owner-width penalty. This is a measured remaining implementation
cost, not a proposed speedup or an accepted fix.

Requested bytes per added helper call, 16-to-32 calls, with only the original
snapshot retained:

| Payload | Entries written | Width 64 | Width 2,048 | Width penalty |
| --- | ---: | ---: | ---: | ---: |
| array | 3 | 16810 | 286634 | 269824 |
| array | 8 | 19735 | 289559 | 269824 |
| array | 64 | 52495 | 322319 | 269824 |
| mixed | 3 | 16976 | 286800 | 269824 |
| mixed | 8 | 20399 | 290223 | 269824 |
| mixed | 64 | 57807 | 327631 | 269824 |
| struct | 3 | 17537 | 295297 | 277760 |
| struct | 8 | 21247 | 299007 | 277760 |
| struct | 64 | 62799 | 340559 | 277760 |

VM and JIT hot-call slopes agree in every group. The width penalty is unchanged
as writes increase from 3 to 64: it indicates a full-owner copy per invocation,
not one full-owner copy per element write, in these tested shapes. Width 64 with
64 writes also covers a dense update; the larger owner uses the same number of
writes. Any replacement storage must be reviewed against both cases.

All 288 allocation observations and 432 VM/interpreter/JIT semantic checks pass
independent coverage, fixture, executable-identity and slope verification.
Cold 1-to-2-call and hot 16-to-32-call controls remain separate. Three cold
width-64/fresh-snapshot/64-write groups allocate one extra request and 52 bytes
per added call in JIT relative to VM; those differences are retained, not treated
as equality or attributed to a runtime cause without further evidence.

Fresh-per-call snapshots are measured separately. Their saved values are checked
after mutation, so isolation remains required. Original and final lengths and
all entries are checked in every fixture. Mixed payloads verify both type changes
and values; the helper takes a runtime write-count parameter.

[Complete observations](array-batch-baseline-2026-10-29.json) embed sources and
identities. Reproduce with `probes/array-batch-isolation.rb`, then independently
verify with `probes/verify-array-batch-isolation.rb`. Future before/after comparisons
must use identical fixture paths. This probe reports allocation traffic, not
retained heap or speed on a clean host.

The next implementation must address general batch storage and profitable
admission, rather than merely increasing the fixed two-entry limit. Existing
row-batch admission provides bounds and primitive-operand checks for nested scalar
rows; root collection payloads need their own validated coverage. Snapshots,
callbacks, partial writes before errors, aliases and mixed types must preserve
ordinary publication boundaries. Sections 3/4 remain open.
