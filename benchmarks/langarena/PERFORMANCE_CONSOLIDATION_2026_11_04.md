# Performance consolidation — 2026-11-04

## Decision and boundary

The user authorized consolidating verified improvements into main for a future
**1.10.0** release, with unfinished candidates explicitly retained elsewhere.
This is not a release: Cargo/package versions remain1.9.0, no tag/artifact publish,
no LangArena submission or maintainer contact. Original branches/worktrees remain.

Integration chain: previous main `2c53503ac037bbeeca87900909872a71dd205bd5`
→ accepted campaign `160f4c9de13660db8c64f6871f8fb222839c4883` (200 commits)
→ retained local-store `c27e3a8b449a406f118a63e35a75e580d7c3c856` (19 more commits)
→ evidence/documentation-only consolidation commits. Every transition preserves
existing ancestry; no conflict resolution or new runtime source was needed.

The integrated runtime is byte-identical across src/Cargo.toml/Cargo.lock to
CI source `79e32f6a37daed7c36bd982afb33fb6567d55d9f`, and across src/Cargo paths to retained checkpointc27e3a8b. Tests retain their
behavior; only an unsafe-inventory test comment points to a relocated report. New evidence/tools/docs do not import scalar
writes, indexed-field fusion, worker owner reuse or old rejected dispatch tactics.

## What main now retains

- Previously accepted field/named-read, loop admission/cache/exit/physical-slot,
  checked-assignment/depth-cache, call-error and ownership correctness work.
- VM-private native-code reclamation and its final platform evidence. macOS/Linux
  reclamation is verified; Windows compatibility is verified but Windows code
  reclamation remains open. Public standalone compiler lifetime is unchanged.
- Consuming whole-function SSA local stores (`bf211fe9`) plus all corrections,
  mixed-value/error/snapshot regression fixtures and acceptance evidence throughc27.
- Existing all-50 benchmark port, comparisons, structural-equivalence ledger and
  production limitations. Checksums alone do not establish equal benchmark work.
- Reusable validated heaptrack decoder, Rust-symbol sampling collector, timing
  verifier with explicit baseline support and byte-identical control collector.
- Subsequent allocation/profile/owner regression evidence selectively copied from
 49c0d989; original research report copied byte-for-byte from the other research
 chat's local artifact. Research SHA256
 `91339b69b90dd709cc8e5ce16e88b8c72a2d9eb7eac93e79230f7b4795beeb53`.
- Newly completed scalar/field first-session receipts and raw timing text in
 `pending-candidate-evidence-2026-11-04.json.gz`; experiment source stays excluded.

## Verification and limits

Existing immutable runtime receipts, independently reviewed before integration:
CI41707246908 has two independent Linux sessions, each1321 library passes,
89 selected integration passes and150 exact checksum successes across VM,
interpreter and JIT. CI41717910054 verifies98 complete sorting arrays per engine,
1764 wrapper observations,1197 Neural states/134064 exact floats,10036 rendered
pixels per variant/runtime and resource controls. CI41719214681 supplies the
frozen sorting A/A/B controls. Their source identity still matches integrated
runtime; no fresh runtime-build claim is made for this consolidation.

Quick JIT computation ratios1.056/1.063 repeat across those sessions; Merge's
benefit is not consistently established. Full-suite aggregate improvement,
production completeness and Rust/Goose parity are NOT claimed. Details and
limitations remain in NATIVE_IMPLEMENTATION_2026_11_03.md.

Local consolidation checks: cargo fmt --check, git diff --check, release-state
and CLI/LSP/language contract-version guards; two heaptrack decoder tests; eleven
timing-verifier positive/negative controls; Ruby syntax for imported collectors;
per-file hash verification of five historical and new evidence archives.
Source equality and ancestry are checked before main is advanced. Broad runtime
compilation was not repeated locally (roughly2GiB free, existing exact-source CI
receipts available). Normal main push CI is additional verification, not release
approval. Its terminal status must be inspected separately if still running.

## Candidates outside main: explicit remaining work

| Candidate | Current tip | Needed before adoption |
| --- | --- | --- |
| Scalar indexed writes |49c0d989 / runtime7f92dc93| Independent timing confirmation, VM Quick/BWT regression disposition, full-state/resource review; then combined-source validation. |
| Indexed scalar field fusion |53b19056 / runtimead7b3fd1| Independent confirmation and complete Neural state/float/snapshot/resource checks; then combined-source validation. |
| Worker native-code reuse |1eea508d / runtimebc7fc048| Explain/mitigate observed VM Neural regression; complete production/resource/platform acceptance. |
| Main scalar-load dispatch |03603253| Clean-host/workload confirmation; no accepted speedup yet. |
| Sparse string-cache dispatch |7e07db7f| Resolve inconsistent workload gains and review cold costs. |
| Root hash/direct read |04a9321e| Rejected workload tradeoff; historical evidence only unless a materially different mechanism is proposed. |

ScalarCI41724170210 and fieldCI41731959897 both succeeded: each1322 library
passes plus selected integrations and all150 checksums. The off-host verifier
rechecks512 observations/64 excluded warmups each, actual alternating order,
binary/build/source/timing identities and exact outputs. Scalar Quick JIT median
computation1.1281x/process1.1109x; VM Quick process0.9795x (0/7 wins), VM BWT0.9752x.
Field Neural JIT computation1.0963x/process1.0952x, with no process median below0.98x
in that16-workload session. These promising first sessions do not close the
remaining gates or establish how candidates perform together. Different hosts;
no aggregate multiplication. Owner Neural control41731130400 remains adverse.

Broader campaign still open: recursive sorting/call overhead, deeper native
coverage, Ray temporary allocation, startup/JIT economics, Brainfuck production,
all-50 structural equivalence/production and clean-host competitor comparison.
These are future engineering/release gates, not missing merges of verified code.

## Full preserved performance branch inventory

No branch is deleted. OUT means its candidate runtime is not on main, not lost.

| Branch | Preserved tip | Disposition |
| --- | --- | --- |
| `codex/borrowed-named-loads-20261031` | `50196d31` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/capture-path-census-20261031` | `94629c2d` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/capture-storage-budgets-20261031` | `e5394ca1` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/checked-assignment-validation-20261101` | `568dc612` | Patch-equivalent implementation integrated as782c712d; no duplicate merge. |
| `codex/checkpoint-review-20261102` | `7fd4f499` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/constant-destination-bound-20261031` | `f58f0a9d` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/depth-cache-hash-20261101` | `9fb0ed70` | Patch-equivalent implementation integrated as55e2f683; no duplicate merge. |
| `codex/jit-owner-reuse-20261103` | `1eea508d` | OUT: preserve worker-reuse experiment; VM Neural regression and production/resource/platform gates unresolved. |
| `codex/loop-break-exit-20261031` | `eb299e0a` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/loop-cache-invalidation-20261101` | `17c3ccd4` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/loop-cache-reuse-20261101` | `348a6270` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/loop-cache-stats-20261101` | `8064895b` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/loop-condition-admission-20261031` | `6d96c151` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/loop-function-fixture-20261031` | `6d96c151` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/loop-reuse-integration-20261101` | `348a6270` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/main-scalar-load-experiment-20261031` | `03603253` | OUT: deferred; clean-host/workload confirmation required. |
| `codex/msrv-lint-compat-20261031` | `2d3ee2ee` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/native-field-fusion-20261103` | `53b19056` | Shared local-store integrated; field fusion remains OUT. Keep for independent timing and complete state/resource validation. |
| `codex/native-implementation-20261103` | `49c0d989` | Local-store through c27e3a8b integrated; later scalar runtime remains OUT. Analysis/evidence selectively copied; keep branch. |
| `codex/native-memory-gates-20261103` | `75445306` | Final implementation already integrated via aab7328f; preserve original candidate provenance. |
| `codex/native-memory-platform-20261103` | `08924627` | Final src/tests/Cargo identical to accepted checkpoint; integrated implementation, preserve original CI history. |
| `codex/native-performance-campaign-20261102` | `160f4c9d` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/native-root-hash-experiment-20261031` | `04a9321e` | OUT: rejected; preserve evidence, do not merge runtime. |
| `codex/nested-dictionary-collections-20261031` | `ac9308ad` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/repeated-array-reservation-20261031` | `1ed18660` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/runtime-census-20261031` | `0b8c9d33` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/string-cache-dispatch-experiment-20261031` | `7e07db7f` | OUT: deferred; consistent workload benefit not established. |
| `codex/struct-field-read-costs-20261031` | `66a59e4e` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/symbolized-kernels-20261031` | `2c53503a` | Already in main ancestry after consolidation; no duplicate merge needed. |
| `codex/vm-code-lifetime-20261101` | `fc358348` | Already in main ancestry after consolidation; no duplicate merge needed. |

## Other historical branches examined

The audit also checked local/remote branches beyond the30 performance branches.
Remote-only heads have no additional ancestry outsidec27. Remaining non-performance
heads are retained; do not treat old release branches as forward runtime merges.

| Historical branch | Assessment |
| --- | --- |
| codex/http-request-header-values | Feature already incorporated by e9e5a677; independent header timeout helper present and subsequently evolved. Node security floor already in59c4b794. |
| codex/native-package-primitives | Package/platform feature incorporated by1778152f; platform.rs byte-identical to branch tip. Generated budgets/history need no old overwrite. |
| codex/kennel-tool-import-isolation | Isolation incorporated byb2299fce/18550ced; contains/recursion-accounting code present. **Unmerged fuzz pinning4c64b915 needs separate assessment**: dated toolchain, cargo-fuzz pin/lock and helper files are absent from current main. Preserve branch; do not blindly adopt September pins. |
| codex/crates-http-dependency | **Registry packaging proposal23eb4ca2 remains separate**; changing dependency identity depends on registry publication/packaging verification. Current vendored HTTP safety patch remains. No package publication authorized here. |
| codex/http-stream-consumer-1.5 / 1.6 | Old-version backports; forward feature already in d6ed9c0c. Keep histories, do not merge old version metadata. |
| codex/quoteflow-pool-fairness-v1.4 | Old1.4 backport; forward FIFO fix0c6f9f89 already in main. Keep historical compatibility evidence. |
| codex/promote-v1.2.1 / codex/release-v1.2.1 / codex/release-v1.7.0-evidence | Historical release/packaging records, not authorization to change current stable version. Keep branches; no forward release metadata merge. |

## Recovery archives for dirty worktrees

`preserved-drafts-2026-11-04.json.gz` stores two exact tracked binary patches,
the untracked capture_census.rs, original worktree paths/HEADs and SHA256 manifest.
Both patches match the current untouched drafts and pass reverse-apply checks.
The archive is inert evidence, not compiled code; after main push it is backed
up remotely. Originals remain dirty intentionally; no edits discarded.

- /private/tmp/kujo-capture-census-20261031: diagnostic counters, not an accepted
  optimization. Recover if further capture profiling is needed.
- /private/tmp/kujo-vm-code-lifetime-20261101: earlier reclamation draft superseded
  by accepted final fix. Preserve as history; do not apply over main.

Archive schema files map paths to base64 contents and SHA256. Decode into a new
recovery directory, verify hashes, and apply tracked patches only to a disposable
checkout of the recorded original HEAD; copy untracked files separately. Never
apply old patches blindly to current main. No cleanup/removal is performed here.

## Session provenance and release boundary

Separate research chat01a19db1-690d-72e1-80ac-2920a3a53687 produced research only;
its report is now retained here. Existing campaign chat01a15c1e-083a-7043-bfc8-634cc21153cb
and Strata reconciliation9568ff46-a42e-4c2c-ae95-f795f9865dc2 provide prior provenance.
This ledger supersedes prior statements withholding integration authorization only
for the accepted campaign/local-store work the user now authorized. It does not
accept unfinished experiments or decide when1.10.0 will be published.

Integration guard repair: the old `notes/VM_MEMORY_CANDIDATE_2026_11_03.md`
violated the field-note naming gate over the full main merge range. Its content
is preserved byte-for-byte under benchmarks/langarena with its test comment
reference updated. The gate is retained; no suppression or runtime change.
