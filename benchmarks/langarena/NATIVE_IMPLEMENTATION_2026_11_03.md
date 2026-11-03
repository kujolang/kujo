# Native implementation campaign

Implementation branch: `codex/native-implementation-20261103`, starting at
`160f4c9de13660db8c64f6871f8fb222839c4883`. Accepted runtime checkpoint remains
`cd5f2b207a8f907d6c263f5a2e405412965344ca`. Remote branch tips match these records;
the separate owner experiment remains at `1eea508d136a3552b6161c110f140de2692e6c8d`.
Neither existing worktree has been modified. No release or accepted-branch merge
is authorized. The new checkout excludes historical JSON archives to fit the
nearly full local disk; runtime builds need CI capacity.

## Delivery gates

- [x] Preserve accepted and experimental identities and create isolated checkout.
- [x] Record 893 accepted source/build/test hashes, frozen executable identity,
  original build-receipt location and local toolchain separately.
- [x] Inspect corrected worker timing artifact and recompute all observations.
- [x] Reproduce and repair Rust v0 profile symbol recognition.
- [ ] A: reduce recursive sorting call/root/access costs; unchanged kernels,
  complete states, snapshots, all runtimes and all50 checksums required.
- [ ] B: reduce measured NeuralNet nested-state cost; retain exact full-state oracle.
- [ ] C: reduce TextRaytracer object/function overhead with identical pixels.
- [ ] D: quantify native/helper/fallback execution on all four priority kernels.
- [ ] E: confirm worker timing regression controls and actual Matmul benefit before
  integrating any owner-reuse source; retain prior isolation/resource/platform gates.
- [ ] F: independently measure startup/frontend/compilation/warmup phases.
- [ ] G: verify other workloads and repair production profiling before attribution.
- [ ] Every candidate: attributable diff, regression tests, all-runtime/all50 gate,
  alternating paired elapsed measurements, independent confirmation and memory review.

No stretch target, overall speedup or completed campaign is claimed here.

## Corrected worker-reuse evidence

Run `41654661543`, source `bc7fc048a8b47fa0a153e4fe7879094af7b7a0d4`, artifact
`13239808137` (`owner-elapsed-linux`). Download digest independently matches
`f617801d00f1b7abab55421153d76884589575e9fb75c00305356df69675948d`.
The existing pinned verifier passes actual executable hashes, optimized build
receipts, source hashes, exact alternating order, all raw outputs and recomputed
ratios: 192 samples, including 24 warmups and 168 measured observations.

| Runtime | Direct | 4 workers | 8 workers | 16 workers |
| --- | ---: | ---: | ---: | ---: |
| VM paired median baseline/candidate | 1.0000 | 1.0047 | 1.0809 | 0.9743 |
| Interpreter | 0.9993 | 0.9844 | 1.0081 | 0.9539 |
| JIT | 2.8269 | 2.3180 | 2.5143 | 2.7470 |

JIT wins all seven pairs in each cell. Its paired ranges are 2.7667–2.8362,
2.0463–2.4336, 2.2175–2.7954 and 2.1682–2.9179 respectively. These are process
times for the worker diagnostic, not LangArena or sorting gains. The interpreter
16-worker cell loses five of seven pairs; its range is 0.9047–1.0240. That control
needs confirmation rather than attributing it to load or silently dismissing it.
Disposition: promising verified JIT result, integration still pending regression
confirmation and unchanged-workload evidence. No experimental source imported.

## Profile repair and limits

Run `41656109318`, source `827c2e73c8be733f0ce893972737b814484225ef`, artifact
`13243316073`. Download digest matches
`20b7f3799cc43a69417947b3c4d83e4f42830c68d5c72d0475a551660336bd43`.
The CLI has Rust v0 symbols, not missing symbols. The old plain-text namespace
regex rejected them. The small isolated Rust maintenance tool `profile-symbols`
uses pinned `rustc-demangle` to resolve both formats while preserving raw samples.
Its regression resolves the actual failed frame to
`<kujo::vm::VM>::execute_dispatch`; unresolved names remain unresolved.

The archived worker thread contains only **two** samples. Symbol repair does not
make this a useful CPU profile. The collector now rejects fewer than 50 worker
samples and retains raw plus resolved output. Neither runtime regression nor
specific hot-helper attribution follows from this failed run. The imported
collector originates from experiment `827c2e73`; runtime source is untouched.

## Fresh sorting entry observations

The accepted frozen CLI SHA-256 is
`17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325`.
Fresh local QuickSort and MergeSort measurement runs both pass checksum 2548773.
QuickSort records 900,732 JIT dispatches and 900,508 native function entries,
including 533,812 owned-argument dispatches. Instrumented wall time is not an
uninstrumented comparison. Allocation/copy counters are explicitly unsupported
by that collector; zero costs must not be inferred from their absence.

The first candidate hypothesis is narrower than the rejected call-transfer and
borrowed-array tactics: native local stores immediately followed by `Pop` currently
duplicate an owned operand into the local, then release the discarded operand.
A verified consuming store could transfer the root instead, removing root creation
and cleanup without changing callable resolution or collection alias semantics.
Actual sort improvement, error cleanup, branch joins, loop persistence and lifetime
proofs remain required before acceptance. Existing dead-parameter and recursive
rebind releases are already present and must not be claimed as new work.

The companion `native-implementation-entry-2026-11-03.json.gz` preserves 425
individually SHA-256-verified files, including raw worker timing/build records,
profile evidence, baseline identities and fresh sorting measurements. Executables
are identified by hash and excluded. Local toolchain: rustc/cargo 1.96.0,
x86_64-apple-darwin; it is not asserted to be the compiler of the frozen binary.
