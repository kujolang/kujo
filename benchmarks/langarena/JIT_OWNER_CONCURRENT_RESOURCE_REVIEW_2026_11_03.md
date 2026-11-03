# Concurrent owners and repeated pools: resource observations verified

Source `53a13d96732e5fa905202597480233c4e0bbbd0c`, successful run
`41647669108`, artifact `13237513656`. Run/job/artifact provenance, pinned source,
build receipt and the diagnostic's executable path match. The executable hash is
a CI receipt; executable bytes are not included in this artifact.

The explicit-lease diagnostic completes eight successive pools, sixteen rounds
per pool, with two workers per round. All 256 native results match. The controller
requires zero idle owners while both workers hold their owners, then exactly two
idle owners after release and join. All 128 active/idle observations and eight
post-destruction observations verify, with exact test membership and success.

Baseline process RSS was 17,880 KiB. Maximum sampled active RSS delta was
3,180 KiB; maximum idle RSS delta was 3,068 KiB. Maximum idle executable mapping
delta was 16 KiB. After each pool's destruction:

| Pool | RSS delta KiB | Executable mapping delta KiB |
| --- | ---: | ---: |
| 0 | 2,880 | 0 |
| 1 | 2,936 | 0 |
| 2 | 2,940 | 0 |
| 3 | 2,940 | 0 |
| 4 | 2,948 | 0 |
| 5 | 3,068 | 0 |
| 6 | 3,068 | 0 |
| 7 | 3,068 | 0 |

This establishes executable reclamation in the observed repeated-pool sequence.
It does not attribute residual RSS between allocator/backend state and worker
thread resources, prove a universal memory ceiling, or exercise two simultaneous
owners filled to every quota. Actual async task routing is covered by separate
integration tests; this diagnostic deliberately controls owner overlap directly.
The distinct-code quota/eviction series remains separate evidence in the
[4,096-task review](JIT_OWNER_4096_RESOURCE_REVIEW_2026_11_03.md).

The same artifact also independently passes 81 targeted Linux tests, all twelve
serial resource cases and forty async economics cells. Seven concurrent-record
controls and six platform-membership controls pass locally. The concurrent path
uses the same test binary as the serial matrix, with no intervening build in the
pinned workflow; successful CI completion establishes the timeout wrapper exited
successfully, not merely that a partial log was uploaded.

`jit-owner-concurrent-resource-review-2026-11-03.json.gz` retains the API metadata,
raw artifact, independent verification receipts and controls with checked content
hashes. These results close the recorded two-owner/eight-lifetime observation
check. They do not establish a speed improvement or whole-candidate acceptance;
paired elapsed, refreshed checksum and production gates remain separate.
