# Proposed performance checkpoint — 2026-11-02

Status: **frozen for review, not tagged or published**. No version bump is made.
The repository remains at 1.9.0; a next-minor release would be 1.10.0.

## Frozen scope

- Checkpoint: `7fd4f499`; accepted runtime change: `782c712d`.
- Original comparison baseline: `f037705190bb6ef765238f9c091086eae0e97485`.
- LangArena inputs: `3c9f84ddaeec834c0631747cb789e8cfd602f993`.
- Goose source: `ea1f368c109c083a2161580ff1d74c70200b6cc8`.
- The bounded sections 3/4 review cutoff is complete. Broader optimization
  opportunities do not extend this checkpoint automatically.
- The section 5 VM-owned JIT code-lifetime candidate is **excluded**. It remains
  in its separate worktree and cannot be included merely because targeted tests pass.
- No further optimization campaign, tag, publication or Kostya contact occurs
  before the user reviews this checkpoint and comparison.

The accompanying `checkpoint-freeze-2026-11-02.json.gz` records 1,390 source/test/
build/vendor file hashes, binary hashes, benchmark input hashes and the existing
CI equivalence check. The binaries are copied and hashed before measurement.
Generated `port.kujo` in the fresh checkout matches the existing port exactly.

## Checkpoint checks

| Gate | Evidence / disposition |
| --- | --- |
| Roadmap release check | Fresh local pass |
| Repository hygiene | Fresh local pass |
| `cargo fmt --check` | Fresh local pass |
| Comparison harness fixtures | Fresh local pass after generating the missing port in the isolated checkout; initial setup failure retained |
| VM language test suite | Fresh local 150/150 pass, 11 skips |
| Interpreter language test suite | Fresh local 150/150 pass, 11 skips |
| Linux CI | Archived run `41294338964`, head `568dc6127556984833617b3cd64914a2307420a2`: seven successful jobs; nine archived artifact hashes independently checked |
| CI applicability | No differences in runtime, tests, Cargo manifests/lockfile, build script, vendor, scripts or workflows between CI head and checkpoint |
| Optimized library coverage | Archived identical-source CI: 1,354 passed, eight existing ignores |
| Fresh complete macOS build/release gate | **Pending**: only about 1.5–1.7 GiB free disk at preflight; no build or deletion attempted |
| New-version artifacts/platform install verification | **Pending**; no version change or release performed |
| Clean-host performance | **Pending**; user has no dedicated host yet |

Archived CI is explicitly reused evidence, not a claim that a new workflow ran.
The language tests above are separate from LangArena's 50 benchmark checksums.
No final release approval follows from the partial local preflight.

## Separate memory candidate

The corrected macOS executable-protection test and other targeted tests pass
(three targeted tests). The candidate library run passes 1,319 tests with eight
existing ignores. Resource, integration, no-JIT, platform and all-runtime
benchmark checks are still required before acceptance.

A freed Cranelift allocation can remain mapped in the system allocator. The
corrected test checks that executable protection is revoked; it does **not**
prove complete deallocation or RSS reclamation. Windows remains a separate
platform limitation. See [VM ownership evidence](VM_CODE_OWNERSHIP_2026_11_01.md).

## Comparison protocol

Seven rotating rounds of all 50 unchanged `test.json` workloads compare original
Kujo VM/JIT, checkpoint Kujo VM/JIT, Rust and Goose: 2,100 attempted samples.
Checksums, process exit, timeout/cancellation, kernel time, process wall time,
peak RSS and host load/memory/thermal observations are retained. Builds and
profilers do not run alongside the comparison.

The original JIT has known correctness failures. Failed or timed-out samples
cannot earn speed results. Zero-rounded competitor times are below reporting
resolution and cannot support speedup ratios. Aggregate ratios must disclose
common passing nonzero coverage; suite sums weight this specific configuration.

These are local **test-size diagnostics**, not production-size or official
LangArena scores. Running on the same laptop improves comparability but does
not equalize changing background load, thermal throttling, memory pressure or
runtime-specific sensitivity. Large repeated gaps remain evidence of real
performance differences; small changes require controlled-host confirmation.

Results are pending completion and independent raw-output verification.
