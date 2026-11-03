# Worker reuse: 4,096-task resource review

Revision `0d6c6316a26a29b4d8c43c0a95ba5d59f7bee2e1`, successful run
`41644765036`, artifact `13236171685`. API provenance, pinned test source,
successful build receipt, exact twelve process invocations, raw log hashes,
sample coverage and counters independently verify. The executable SHA remains
a CI receipt: this resource artifact does not include the executable bytes.

All twelve cases complete successfully. Each checks native results and requires
one native boundary per task. Same-code pooled cases compile once; distinct-code
and unpooled cases compile once per task. Values below are KiB differences from
each isolated process's own warmed baseline. Maxima are sampled every 32 tasks,
not OS peak memory. Different rows are different processes, not paired deltas.

| Storage | Code | Tasks | Max sampled RSS | RSS after drop | Max executable mappings | Executable after drop |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Pooled | Same | 32 | 372 | 436 | 8 | 0 |
| Pooled | Same | 1,024 | 460 | 460 | 8 | 0 |
| Pooled | Same | 4,096 | 432 | 432 | 8 | 0 |
| Pooled | Distinct | 32 | 568 | 632 | 256 | 0 |
| Pooled | Distinct | 1,024 | 8,744 | 8,416 | 4,096 | 0 |
| Pooled | Distinct | 4,096 | 8,724 | 8,236 | 4,096 | 0 |
| Unpooled | Same | 32 | 508 | 508 | 0 | 0 |
| Unpooled | Same | 1,024 | 568 | 492 | 0 | 0 |
| Unpooled | Same | 4,096 | 744 | 460 | 0 | 0 |
| Unpooled | Distinct | 32 | 584 | 584 | 0 | 0 |
| Unpooled | Distinct | 1,024 | 712 | 612 | 0 | 0 |
| Unpooled | Distinct | 4,096 | 728 | 468 | 0 | 0 |

In the pooled distinct 4,096-task process, checkpoints at 512-task intervals
show RSS deltas of 8,492, 8,556, 8,640, 8,664, 8,648, 8,648, 8,724 and 8,684 KiB.
Executable deltas at those points are 4,096, 4,088, 4,080, 4,072, 4,064, 4,056,
4,048 and 4,040 KiB. Every pooled checkpoint has one idle owner; every unpooled
checkpoint has zero. The distinct-code sequence exercises repeated quota
eviction while preserving results. This bounded series does not establish a
universal total-heap bound or behavior over arbitrarily many sessions.

All executable mapping deltas return to zero after destruction. RSS does not:
the 4,096 distinct-code pooled process retains an 8,236 KiB delta. This evidence
does not attribute that remainder to live pool objects versus allocator/backend
retention. The pool trades additional residency for fewer repeated compilations;
neither a leak nor an elapsed speed benefit is inferred solely from this table.

The same artifact independently verifies 81 targeted Linux tests and forty
async economics cells. JIT serial 16- and 64-job cases each compile once.
Concurrent 4/8/16-worker cases compile 5/5/8 times for sixteen jobs and 11/12/14
times for sixty-four jobs. Scheduling affects these counts; they are not speed
ratios. Nineteen resource-verifier controls and six platform-membership controls
pass locally.

The adjacent `jit-owner-4096-resource-review-2026-11-03.json.gz` preserves API
metadata, raw artifact files, verification receipts and controls with checked
content hashes. Concurrent two-owner/repeated-pool observations are a separate
pending run. Paired elapsed, remaining platform and production gates still
prevent whole-candidate acceptance.
