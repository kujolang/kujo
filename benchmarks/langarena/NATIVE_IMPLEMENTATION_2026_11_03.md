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

## First implementation: retained on the isolated branch

`bf211fe9` implements consuming SSA local stores only for whole-function native
code when the next opcode is `Pop`. Initialization/error checks run first; the
operand transfers to the local and a Null placeholder preserves stack shape.
Loop physical slots, peek stores and named bindings keep their existing paths.
The compiled regression compares equivalent consuming/peek paths and requires
one fewer issued root per store, unchanged backing storage and complete cleanup.

CI `41707246908`, source `79e32f6a37daed7c36bd982afb33fb6567d55d9f`, runs two
independent Linux sessions. Both completed library/integration checks, exact
all150 checksums and 16-workload paired timings; their raw evidence is reviewed.
The earlier run `41706406972` was cancelled after discovering its requested
all-runtime test name did not yet exist on the accepted branch. It is not an
acceptance receipt. `79e32f6a` adds that explicit test and rejects zero-test gates.

Session 2 is now independently reviewed (artifact `13269802619`): 1,321 library
passes, eight explicit manual-test ignores, 89 selected integration passes, two
ignored full-checksum tests, then the explicit all150 gate passes every exact
name/checksum in all three runtimes. All 916 recorded source hashes match the
pinned Git revision. Actual executable bytes, release build receipts, unchanged
timing boundaries and all 512 alternating observations verify.

Session-2 JIT computation paired medians are 1.063x QuickSort, 1.044x MergeSort,
1.050x NeuralNet and 1.046x TextRaytracer. Their paired ranges are respectively
1.051–1.085, 1.021–1.055, 1.013–1.057 and 0.997–1.074. Process medians improve
1.051x/1.042x/1.044x/1.049x. This is provisional single-session evidence, not
acceptance or a competitor comparison. VM Matmul T4/T8/T16 process medians are
0.951x/0.978x/0.963x; T16 loses seven pairs. Independent-session agreement and
regression/resource review remain required. Raw timings and verification are in
`local-store-session-2-2026-11-03.json.gz`.

Session 1 (artifact `13269309023`) independently passes the same 1,321 library,
89 integration and 150 exact checksum outputs, with the same 916 source hashes
and toolchain. Both sessions' binary bytes are verified separately; their build
hashes differ. Session 1 reports EPYC 7763, session 2 EPYC 9V74. Absolute timings
are not pooled across hosts/builds.

| JIT computation paired median | Session 1 | Session 2 |
| --- | ---: | ---: |
| QuickSort | 1.056x | 1.063x |
| MergeSort | 1.005x | 1.044x |
| NeuralNet | 1.021x | 1.050x |
| TextRaytracer | 1.038x | 1.046x |

QuickSort wins all fourteen computation pairs. MergeSort's improvement does not
repeat clearly in session 1. Session-1 VM Quick/Merge process medians are
0.921x/0.938x, unlike session 2's 1.001x/1.025x. These differences required
additional controls. Run `41719214681` at `984da2ed` uses frozen session-1
binaries plus a byte-identical baseline copy in all six three-way orders twice
for VM sorting. This tests measurement variation directly; it is not permission
to dismiss regressions. The collector's 78-invocation synthetic control verifies
exact order balance, not performance.

Follow-up run `41717910054` at `3feb22a7` was dispatched after successful
completion. It downloads the frozen candidate, verifies source/runtime identity,
checks expanded all-type/branch/error fixtures in all three runtimes, executes the
complete sorting oracles and the 1,197-state / 134,064-float NeuralNet control,
and measures resources/native entries separately from elapsed timing. NeuralNet
tools are imported unchanged from `1eea508d`; no experimental runtime or port
correction is imported with them. Results and the bounded disposition follow.

The follow-up also compares all 10,036 rendered pixels across three image sizes
in all runtimes, and collects Heaptrack allocation profiles in separate runs of
the four priority workloads. Raw traces and analysis are retained. Allocation
counts, retained heap, peak RSS and logical root issuance are distinct measures;
none supplies a copied-byte total or proves an elapsed-time improvement. Stripped
executable stacks may limit attribution.

### Frozen state and resource follow-up passed

Run `41717910054`, artifact `13272195277`, completed successfully. Frozen binary
hashes match session 1 before and after execution. Off-host raw comparisons verify
98 complete sorting arrays per engine, 1,764 wrapper/RNG/input observations,
1,197 complete normalized NeuralNet states (134,064 compared float values), and
10,036 pixels per baseline/candidate/runtime. Expanded local-store fixtures pass
VM/interpreter/JIT. Existing snapshot assertions and negative controls remain.
This does not close all-50 structural equivalence or production acceptance.

Heaptrack 1.3.0 reports the following whole-process allocation calls, separately
from timing. Counts include frontend, compilation, preparation and warmup.

| JIT workload | Baseline allocation calls | Candidate allocation calls | Baseline/candidate peak heap |
| --- | ---: | ---: | --- |
| QuickSort | 1,007,735 | 1,004,982 | 40.00M / 39.92M |
| MergeSort | 1,663,686 | 1,659,316 | 58.86M / 58.14M |
| NeuralNet | 11,830,983 | 11,331,648 | 25.91M / 24.78M |
| TextRaytracer | 5,464,577 | 5,120,953 | 37.73M / 34.70M |

VM Quick/Merge/Neural allocation counts are identical; ray differs by three
calls out of 9.67 million. Reported unfreed-at-exit memory is unchanged at the
displayed precision (JIT 19.23K). This is not a universal no-leak proof or a
copied-byte measurement. Stripped stacks prevent reliable helper attribution
from these allocation traces. Heaptrack-instrumented elapsed times are excluded
from speed comparisons.

Separate runner phase records reproduce off-host: frontend parse, bytecode
compile, inclusive JIT compilation, creation, preparation, warmup and configured
iterations remain distinguished. Native function-entry counts match baseline and
candidate in all four workloads. Compiler spans overlap execution phases; no
sum or subtraction is presented as an independent compilation measurement.

`local-store-state-resources-2026-11-03.json.gz` preserves 194 hash-verified text
records and SHA-256/size identities for 16 compressed allocation traces and two
Rust oracle executables. Binary files remain in the downloaded/remote artifact,
not duplicated into Git. Off-host state and phase verification are retained;
allocation totals are Heaptrack's summaries, not a second raw-trace decoding.

### Frozen VM sorting identity controls passed

Run `41719214681`, artifact `13272861200`, completed successfully. Independent
review verifies all 78 invocations, six warmups, all six three-way orders twice,
actual frozen executable hashes, unchanged timed port and exact raw outputs.
The two baseline labels use byte-identical executables. Candidate ratios below
compare against the within-triplet geometric mean of those two baselines.

| VM paired median | Identical baseline A/B | Baseline reference/candidate | Candidate range |
| --- | ---: | ---: | ---: |
| Quick computation | 1.0132x | 1.0033x | 0.9867–1.0199 |
| Quick process | 1.0160x | 0.9981x | 0.9837–1.0069 |
| Merge computation | 1.0155x | 0.9931x | 0.9728–1.0194 |
| Merge process | 1.0173x | 0.9980x | 0.9718–1.0172 |

The earlier large VM sorting loss does not reproduce with the same binaries.
Identical binaries themselves differ measurably in this experiment. This does
not establish the cause of earlier differences, justify correcting their times,
or prove universal regression freedom. Matmul's inconsistent VM deltas also
remain part of the full-suite review rather than being silently discarded.

Disposition: retain `bf211fe9` on the isolated implementation branch as a bounded
JIT improvement, supported by repeated QuickSort computation gains, full-state
and all-runtime checksum gates, and resource controls. Do not merge it into the
accepted branch yet. No clear repeated MergeSort improvement, all-50 aggregate
speedup, production acceptance or Rust/Goose parity is established. The candidate
removes one root issuance per eligible store; copied-byte volume remains unmeasured.

`local-store-sort-controls-2026-11-03.json.gz` preserves raw observations,
source/run provenance, the off-host verifier, recomputed comparisons and download
digest. Every embedded file is hash-verified. This closes the requested frozen
sorting control, not the broader performance campaign.

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

### Independent confirmation

Run `41654661543` attempt **2**, same pinned source, artifact `13265972965`.
Downloaded ZIP SHA-256:
`71fd661301bc8402d676e2fecee0dd084dbd43646303007588be5c7cab9a3011`.
All 192 samples, raw outputs, actual binaries and alternating orders independently
verify again. Toolchain and reported CPU model match the first session; executable
hashes differ between builds and are recorded separately. Timings are paired
within each execution session, not mixed between hosts/builds.

| Runtime | Direct | 4 workers | 8 workers | 16 workers |
| --- | ---: | ---: | ---: | ---: |
| VM paired median baseline/candidate | 1.0146 | 1.0160 | 1.0030 | 0.9844 |
| Interpreter | 1.0008 | 1.0166 | 0.9849 | 1.0162 |
| JIT | 2.8289 | 2.3509 | 2.4285 | 2.3814 |

JIT wins all 28 pairs here, 56 across both sessions. The interpreter 16-worker
loss does not repeat (four candidate wins in seven pairs). VM 16-worker medians
remain slightly below parity in both sessions, with broad overlapping paired
ranges (second session 0.8886–1.2099); this remains explicit regression review,
not a claimed speedup or proven causal loss. Actual unchanged-workload testing
is required before integration. The `owner_workloads` workflow uses these frozen
experiment binaries with the accepted port, separately from the local-store
candidate. The companion confirmation archive retains raw samples and receipts;
no global LangArena gain is inferred from this worker diagnostic.

### Unchanged-workload gate: integration blocked

Run `41711233289` attempt 1, harness `c54a309f`, artifact `13268447748`, uses
the attempt-2 frozen runtime binaries. All 512 observations independently verify:
64 excluded process warmups and seven alternating pairs per workload/runtime.
Matmul T4/T8/T16 JIT process paired medians improve 1.222x/1.358x/1.532x, with
seven wins each. Computation medians improve 1.385x/1.684x/1.987x. These are
test-size observations on this Linux host, not a production or aggregate gain.

VM QuickSort and MergeSort lose all seven pairs: process baseline/candidate
medians are 0.892x and 0.929x (candidate about 12% and 8% slower). Computation
medians are 0.885x and 0.930x. BWTEncode VM computation also loses all seven
pairs, median 0.959x. The four priority JIT process medians are 1.005x, 0.998x,
1.007x and 0.989x for Quick, Merge, NeuralNet and TextRaytracer respectively;
there is no established priority-workload improvement. Complete raw times,
paired ranges, outputs and hashes remain in the archive below.

These losses blocked integration. Attempt 2 of the same run is now independently
verified (artifact `13269744010`): 512 observations, identical frozen executable
bytes, exact order and raw outputs. Matmul JIT process medians again improve
1.202x/1.392x/1.557x, with seven wins each. The VM sorting losses do not repeat:
Quick/Merge process medians are 1.021x/1.016x, with seven/six wins. VM NeuralNet
instead loses seven process pairs, median 0.983x. Neither averaging away a loss
nor selecting the faster session establishes regression freedom. Source
inspection shows task-pool allocation/checkout gated by JIT, so the cause of
the VM losses is unresolved. Compiler layout effects are a hypothesis, not a
diagnosis. The existing experiment is still preserved separately.

`native-independent-confirmations-2026-11-03.json.gz` retains both second-stage
reviews, source/provenance records and raw observations: 2,094 individually
hash-verified files, with executable copies excluded. Worker integration and
accepted-branch integration remain pending; no full-suite aggregate gain is claimed.

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

The repaired collector completed in `41713260539`, harness `753afcfb`, artifact
`13269950946`, using the original frozen symbolized executable SHA
`629acef99b0963da1004b6356e6c0693cd103aa1bfb184031a3f5d27b741e274`.
Independent reconstruction of every demangled sample matches exactly. VM,
interpreter and JIT contain 1,475, 1,583 and 1,566 worker samples after the
1,000 checked warmups. Inputs match the original production fixture. Every
partial production execution was terminated after sampling, not completed.

The JIT sample identifies root encode/release/clone work, value movement and
environment lookup among active leaf frames. VM samples include opcode cloning;
interpreter samples include allocation, environment lookup and expression
evaluation. The two idle controller threads dominate all-thread wait counts;
those counts must not be interpreted as worker CPU bottlenecks. These observations
justify further attribution, not a specific optimization or a speed estimate.
They describe the original experimental executable, not the local-store candidate.

`owner-workloads-bf-profile-2026-11-03.json.gz` retains both raw artifacts,
independent verification receipts, provenance and ZIP digests, excluding binary
copies. Every archived file hash was checked after writing.

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

Additional accepted-binary local diagnostics pass NeuralNet checksum 1586674334
and TextRaytracer checksum 1438660. The four instrumented observations are:

| Workload | JIT dispatches | Native function entries | Owned-argument dispatches | JIT compilation entries |
| --- | ---: | ---: | ---: | ---: |
| QuickSort | 900,732 | 900,508 | 533,812 | 5 |
| MergeSort | 1,000,033 | 999,808 | 600,010 | 6 |
| NeuralNet | 156,082 | 156,010 | 156,020 | 8 |
| TextRaytracer | 1,110,423 | 1,109,608 | 742,465 | 10 |

These are process-wide event counts, including preparation and warmup. They
demonstrate frequent native entries, not the fraction of time or instructions
executed natively. Native functions can still call costly runtime helpers. The
recorded parse, bytecode compile and inclusive JIT compile spans are separate
diagnostic observations; subtracting them from wall time does not isolate warmup.
All four raw observations, outputs and port hashes are preserved in
`priority-baseline-observations-2026-11-03.json.gz`. A separate captured-value
diagnostic records 136 shallow bytes for one cell on this frozen executable;
this is a value-size observation, not allocation or CPU attribution.

The companion `native-implementation-entry-2026-11-03.json.gz` preserves 425
individually SHA-256-verified files, including raw worker timing/build records,
profile evidence, baseline identities and fresh sorting measurements. Executables
are identified by hash and excluded. Local toolchain: rustc/cargo 1.96.0,
x86_64-apple-darwin; it is not asserted to be the compiler of the frozen binary.

## Next isolated experiment: scalar native indexed writes

Hypothesis: integer-index/scalar-value writes needlessly decode and move two
full-width dynamic Values before destination lookup. The candidate defers those
materializations, then uses the existing array COW/bounds rules; nonarrays and
owned operands retain generic VM behavior. Invalid kinds still fail operand
validation before destination lookup. This is an experiment, not an accepted gain.

The baseline for this experiment is `c27e3a8b449a406f118a63e35a75e580d7c3c856`,
which already includes the local-store improvement. A separately linked identical
Rust helper probe isolates this mechanism; unchanged 16-workload VM/JIT timings
and all150 checksums determine whether to retain it. Probe gains alone are
insufficient. Exact-bit NaN/signed-zero, negative/extreme index, unique/shared
array and all three destination-location unit controls compare directly with the
VM write contract. A language fixture covers snapshots, errors, owned fallback,
dictionary and byte writes in all three runtimes. It passes on the accepted Mac
binary before the candidate build. No candidate test result is yet claimed.

### Independent allocation trace reconstruction

The standalone Rust maintenance decoder `probes/heaptrack_histogram.rs` reads the
interpreted v3 stream used by Heaptrack 1.3.0. Its record interpretation follows
[the pinned upstream parser](https://github.com/KDE/heaptrack/blob/v1.3.0/src/analyze/accumulatedtracedata.cpp).
Positive and malformed-stream controls pass. All 16 original traces independently
reproduce the exact allocation-call totals printed by Heaptrack. This supersedes
only the earlier limitation that allocation totals lacked an independent raw
trace decode; stripped stacks still limit attribution.

| Local-store JIT candidate | Allocation calls | Requests <=16 bytes | Cumulative requested bytes |
| --- | ---: | ---: | ---: |
| QuickSort | 1,004,982 | 932,247 | 135,073,707 |
| MergeSort | 1,659,316 | 1,565,930 | 282,398,838 |
| NeuralNet | 11,331,648 | 9,597,579 | 1,015,148,526 |
| TextRaytracer | 5,120,953 | 3,432,458 | 463,761,945 |

These are whole-process request totals, not retained memory or copied bytes.
NeuralNet's 6/7/8-byte buckets contain 4,084,131 / 2,282,456 / 1,852,051 requests;
ray's 4-byte bucket contains 1,627,769. Name/field copying is a source-supported
hypothesis for further isolation, not proven attribution from allocation sizes.
The archive `local-store-allocation-histograms-2026-11-03.json.gz` preserves all
16 histograms, input trace/summary hashes, decoder source and provenance.

Scalar-write CI `41722996389` stopped at test compilation because the new test
incorrectly assumed `Value: PartialEq`. The assertion-only correction compares
scalar kinds explicitly and retains exact float-bit checks. No candidate runtime
success or timing result is claimed from that failed run.

### Priority helper attribution follow-up

`native-priority-profile.yml` reuses the original symbolized experiment executable
SHA `629acef9…` from run `41656109318`, rather than pretending that stripped current
CLI traces contain named helpers. It requires unchanged priority kernel inputs
and identical `assignment.rs` / `value.rs` source, checks all four test-size
outputs in VM/JIT, resolves Rust symbols and rejects fewer than 50 worker samples.
Samples begin only after each warmup marker. The separate hosted Mac runner does
not share the scalar comparison's Linux host.

This executable predates the local-store/scalar-write changes and includes the
separate worker experiment. Its samples can guide investigation of unchanged
shared helpers; they cannot establish current-candidate performance, native
instruction coverage or exact native-versus-helper CPU fractions. No runtime or
benchmark algorithm change accompanies the profiling follow-up.
