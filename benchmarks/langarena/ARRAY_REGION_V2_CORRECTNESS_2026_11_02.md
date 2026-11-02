# Revised borrowed array regions: correctness receipt

This is an unaccepted performance candidate. It retains validated root-slot hints
across borrowed-view invalidation and uses those hints for generic fallback.
It still calls a Rust element helper; direct generated loads and the broader
native-access goal remain unfinished.

The comparison CLI SHA-256 is
`acf5569da1fa6d789240aedbce813558aa387318da38f46dd064a9933b03471f`.
Its preserved copy is `region-v2/candidate-kujo` under local campaign evidence.
The source-bound verification checks 1,396 source/input identities.

| Gate | Result |
| --- | --- |
| Selected region unit regressions | 3 pass |
| Broader JIT unit tests | 93 pass, 7 ignored |
| Native ownership unit tests | 20 pass |
| Cargo release integration test | Pass; two fixtures in VM/interpreter/JIT |
| Same integration test bound to the preserved comparison CLI | Pass; all six fixture/runtime combinations |
| Unchanged test-size workloads on preserved CLI | 150/150 pass; 50 per runtime |
| Exact registry, config, checksum, exit and truncation checks | Pass |
| Formatting and patch whitespace | Pass |
| Paired speed, independent session, production and clean-host acceptance | Pending |

The original gate correctly rejected binary drift. Cargo integration produced
CLI `5275bf99…`, replacing the earlier CLI `acf5569d…`. Its checksum runs passed,
but those results alone do not verify the executable selected for comparison.
Both binaries and the original failure are retained.

The baseline's archived build command selects `--bin kujo` without `--lib`.
Running `cargo build --release --locked --bin kujo -j1` on the candidate completed
successfully and restored exactly `acf5569d…`. We then compiled the unchanged
std-only integration test with its CLI path fixed to that preserved executable
and JIT coverage enabled, ran it, and repeated all 150 checks against that same
executable. Identity checks bracket the runs. No result from the other executable
is substituted for these checks.

The original Cargo integration took 1,896.7 seconds including rebuilding. Its
supervision extension is retained; it was not restarted or scored as a benchmark.
The comparison build subsequently completed from cache in 1.83 seconds.

`array-region-v2-correctness-2026-11-02.json.gz` retains 23 independently
hash-verified artifacts: source-bound states, exact reports/logs, both integration
routes, build provenance, the original identity rejection, the supervision
extension, and the unaccepted candidate patch. The archive includes
`verify-release-artifact.rb`, which asserts exact coverage and output, not merely
runner exit status. Elapsed values from checksum runs are diagnostic only.

No runtime change is accepted by this receipt. Paired target and fallback
measurements must still establish whether this sole hypothesis revision is
worth retaining.
