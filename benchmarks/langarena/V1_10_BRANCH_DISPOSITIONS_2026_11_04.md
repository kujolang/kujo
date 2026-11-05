# 1.10 preserved-work disposition — 2026-11-04

This is the final **scope decision** for preserved branches, not release approval.
Every local and origin branch fetched for this audit has a disposition below.
No branches or worktrees were deleted; no experimental runtime was imported.
Main already contains the accepted campaign and local-store improvements.
The audit base is `4e4adcb51affbc6ca0845f774e634e2535522af2`; subsequent
`6731e8fc` fixes the Clippy gate without changing runtime behavior and
`2984bb9e` adds extension dependency compatibility controls.

## Closed decisions

- Seven proposals are deferred beyond 1.10: scalar writes, field fusion,
  worker code reuse, main scalar loads, string dispatch, fuzz pinning, registry
  HTTP packaging. Their missing evidence is not waived or claimed complete.
- Root-hash/direct-read experiment is rejected on recorded workload regressions.
- Two preserved dirty drafts: capture census is deferred as diagnostic tooling;
  old VM reclamation draft is rejected as superseded by the accepted final fix.
  Both remain recoverable from `preserved-drafts-2026-11-04.json.gz` and original
  worktrees. Main is clean; those worktrees intentionally remain dirty.
- Patch-equivalent and superseded branches require no duplicate merge. Historical
  release/backport branches are retained as archives, not forward merges.
- No further branch-selection decisions remain for 1.10. This does not mean all
  optimization goals, benchmark equivalence, production runs or release gates pass.

## Fact-check method and evidence

Each ref was checked with merge-base against main. Non-ancestors were checked
using cherry equivalence, unique source/test/workflow logs, candidate reports,
source identity and the prior consolidation evidence. No unknown unclassified
ref is allowed by the inventory generator. Full object IDs below freeze scope.
See [consolidation evidence](PERFORMANCE_CONSOLIDATION_2026_11_04.md) for CI IDs,
raw archive locations, accepted source identities and measured candidate limits.
Scalar/field receipts remain first-session evidence, not acceptance. Fuzz4c64b915
and registry23eb4ca2 were re-read rather than assumed integrated. Memory-platform
src/tests/Cargo equality and assignment/depth patch equivalence were rechecked.

## Extension update: blocked, explicitly deferred

The requested minimatch update is **not merged**. Locked10.2.5 remains affected
by [GHSA-g3hv-75wg-m249](https://github.com/advisories/GHSA-g3hv-75wg-m249).
A scoped11.0.4 override makes npm audit clean but breaks the language client's
`makeRe()` API. Latest published client11.0.0 still uses that API and depends on
10.x. No compatible patched10.x release was found as of this audit.
Both current client callers use default optimization level1; advisory amplification
requires level2. Extension exploitability is unproven, not declared impossible.
The unsafe override was reverted. Added dependency compatibility controls pass
on the current tree and fail on an isolated11.0.4 override. Static checks alone
previously missed that break. Package versions/lockfile are unchanged; audit
still reports two moderate entries for the affected dependency chain.

A compatible upstream release, reviewed backport, or client adaptation is a
separate follow-up. Do not call the extension advisory fixed or publish an
updated extension as security-remediated. No full VS Code host test was run.

## Release boundary

Version remains1.9.0;1.10 is proposed, not tagged or published. Clean-host competitor
comparison, all-50 structural equivalence, Brainfuck production completion and
Windows native-memory reclamation remain disclosed limitations/future work.
Native/npm runtime release scope must be distinguished from VS Code and Cargo
registry distribution. Registry packaging deferral means no claim of a verified
Cargo registry release. Release authorization and final terminal CI review remain
required before publication.

CI41961515306 failed only the constant-assertion Clippy lint in the ignored all-runtime
checksum test. The guard remains runtime-enforced using explicit panic; runtime source
is unchanged. New main CI must confirm the repair; do not treat pending as passed.

## Complete frozen ref inventory

140 refs checked individually (local plus origin; aliases intentionally listed).

| Ref | Full tip | Final disposition | Evidence / reason |
| --- | --- | --- | --- |
| `codex/assetworks-confined-digest` | `cc2d7dbb59a8dc05f00d629e100932f56f4062f6` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/bluepencil-confined-directory-pages` | `2a4dbe169cd889856fa6d762899f7f8892027327` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/borrowed-named-loads-20261031` | `50196d3151e7e28759bc9b80b748ed8b1f357ab1` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/capture-path-census-20261031` | `94629c2db388695f7cf148af3b47c78fa4e34267` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/capture-storage-budgets-20261031` | `e5394ca1e7a82958210243b19b70d18581d5fc94` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/checked-assignment-validation-20261101` | `568dc6127556984833617b3cd64914a2307420a2` | INCORPORATED | Patch-equivalent to 782c712d; git cherry reports minus. |
| `codex/checkpoint-review-20261102` | `7fd4f499391b9c63b9910426fb68404bb99bb29f` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/consolidate-performance-20261104` | `d6f530bc8b038c0b16ca0ae86aad98f07f0b7ac9` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/constant-destination-bound-20261031` | `f58f0a9deb40c751d55121fcbb69495bc724b7b4` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/crates-http-dependency` | `23eb4ca2d2c8ef53f8dc22517aee36fd55402ad1` | DEFER BEYOND 1.10 | 23eb4ca2 registry-identity proposal requires published reviewed fork and normalized root-package verification; no publication authorized. Keep vendored HTTP safety patch. |
| `codex/depth-cache-hash-20261101` | `9fb0ed705c59a8da6c6eab36e9f50c3a0722f728` | INCORPORATED | Patch-equivalent to 55e2f683; git cherry reports minus. |
| `codex/dispatch-release-receipt` | `8a84fb7ce4d1dd3abaf290736939f1b7883b4d03` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/email-bounded-xml` | `1e20cf9ba5ca43dc3d63ea4016d89f934a805f17` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/graph-attempt-accounting-docs` | `2946f2a1a76700519292db3da8004ea1a9e31f61` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/http-request-header-values` | `bdb9d4c92b01bf686bceef60adad09148aaca128` | INCORPORATED | Feature e9e5a677, evolved timeout helper, Node floor 59c4b794 present. |
| `codex/http-stream-consumer-1.5` | `302bbb19ac4fb661843e4e2e3d633f7bae5d794b` | ARCHIVE / NO MERGE | Older-version backport; forward implementation d6ed9c0c in main. |
| `codex/http-stream-consumer-1.6` | `92e0e0a8125da5d005c1e4f5f9a9784f436158a9` | ARCHIVE / NO MERGE | Older-version backport; forward implementation d6ed9c0c in main. |
| `codex/jit-owner-reuse-20261103` | `1eea508d136a3552b6161c110f140de2692e6c8d` | DEFER BEYOND 1.10 | VM Neural adverse control; production/resource/platform gates incomplete. Keep current implementation excluded. |
| `codex/json-float-roundtrip` | `b25b4edd7f882810ab29a58028ad4d8de9fcaca9` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/kennel-tool-import-isolation` | `66029f26105ed9206d88c3560790cf65b5bf1d28` | DEFER BEYOND 1.10 (fuzz proposal only) | Isolation/type fixes incorporated. 4c64b915 fuzz pinning has September toolchain/lock and absent helpers; requires current-toolchain validation. Existing fuzz failure is not acceptance evidence. |
| `codex/kujo-1-5-release-record` | `59c4b794070fe1f1c4d690b1639aa4357f90809a` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/kujo-1-8-core-quality` | `59c4b794070fe1f1c4d690b1639aa4357f90809a` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/launch-cleanup` | `7391b99f6fbb349feea8be0e6e6e7b8a7ca55aef` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/loop-break-exit-20261031` | `eb299e0a699560c80d87d0d81582914ba2c6d956` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/loop-cache-invalidation-20261101` | `17c3ccd46a685ac25a53e8f0f1028e3de7524931` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/loop-cache-reuse-20261101` | `348a62700179e59beccaefddc7ed4da30992d67b` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/loop-cache-stats-20261101` | `8064895bc10293275e4cd01f29642ee5cc5685bd` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/loop-condition-admission-20261031` | `6d96c151e8b9f3d814043eebe5768ae5c61cde97` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/loop-function-fixture-20261031` | `6d96c151e8b9f3d814043eebe5768ae5c61cde97` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/loop-reuse-integration-20261101` | `348a62700179e59beccaefddc7ed4da30992d67b` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/main-scalar-load-experiment-20261031` | `036032535ad4bdf540e9c1b180c33dae3f34d517` | DEFER BEYOND 1.10 | No repeatable accepted workload improvement; main-only variant needs independent confirmation. |
| `codex/markdown-doc-cleanup-pinned-web` | `59c4b794070fe1f1c4d690b1639aa4357f90809a` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/msrv-lint-compat-20261031` | `2d3ee2ee6649df6074873d2ec378f25beaeb9ffe` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/native-absolute-path` | `6a0c9e3949cae246a6abbccbbe11e7c1e0bc4b28` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/native-field-fusion-20261103` | `53b190560217d66ec2b31742ec6affa49a5126ec` | DEFER BEYOND 1.10 | One passing session; independent timing, full Neural state/resource and combined-source gates missing. |
| `codex/native-implementation-20261103` | `49c0d98918bd52dd43639e8866f3a7b815801f77` | DEFER BEYOND 1.10 | Local-store accepted through c27; later scalar writes lack second session/full-state/resource acceptance and have VM Quick/BWT adverse observations. Evidence/tools already retained. |
| `codex/native-memory-gates-20261103` | `754453061a98fe3c994b89f1c9308326a50c544d` | INCORPORATED | Final implementation integrated via aab7328f; original candidate provenance retained. |
| `codex/native-memory-platform-20261103` | `089246278e955545d98415f66b851b01803a12ce` | INCORPORATED | src/tests/Cargo tree equals accepted 160f4c9d; platform workflow changes integrated. |
| `codex/native-package-primitives` | `4c408da54797ad8e22a878d328c836513bc15068` | INCORPORATED | 1778152f includes platform primitives; platform.rs identical to candidate. Do not overwrite evolved budgets. |
| `codex/native-performance-campaign-20261102` | `160f4c9de13660db8c64f6871f8fb222839c4883` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/native-root-hash-experiment-20261031` | `04a9321edc5ed718404c083d73d63549ee1ac98a` | REJECT | Repeated uncached-read/Merge VM regressions; preserve evidence, do not adopt this mechanism. |
| `codex/nested-dictionary-collections-20261031` | `ac9308ad6b8695e0ad07b349558039e9aae2c52d` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/operator-readiness-docs` | `59c4b794070fe1f1c4d690b1639aa4357f90809a` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/promote-v1.2.1` | `817c632be7d0b0cfa2f9c1d1f52c6a0c0e0f492c` | ARCHIVE / NO MERGE | Historical release metadata; not a forward runtime change. |
| `codex/quoteflow-pool-fairness-v1.4` | `f1a45d3b286471aa9d7b164a6baaa3dbde34dff5` | ARCHIVE / NO MERGE | Forward FIFO fix 0c6f9f89 in main; retain old release/PDF compatibility history. |
| `codex/release-consumer-checklist` | `334567c0c11f7530f9087652f982cc443a93c495` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/release-v1.2.1` | `6c9eb286dd799d00c348f58da620472139659818` | ARCHIVE / NO MERGE | Historical release/Windows portability history, superseded by current platform releases. |
| `codex/release-v1.3.0` | `59c4b794070fe1f1c4d690b1639aa4357f90809a` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/release-v1.3.1` | `dc4803598d0421b31ecfd3f1027732589f1e8df1` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/release-v1.4.0` | `1a7b6ef0eacd4918912ef58cfb5ec8533bff9e91` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/release-v1.7.0` | `f68750b368e6ebaf4dcaf588b4984651bb4d9670` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/release-v1.7.0-evidence` | `f1a220f2f0c2bf6d6df5732b8ef06b6b7bc7a04d` | ARCHIVE / NO MERGE | Historical registry exception/release evidence; no new runtime delta. |
| `codex/repeated-array-reservation-20261031` | `1ed1866006bdf7b2666e819689b1228613f7807a` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/runtime-census-20261031` | `0b8c9d33909b52d17028508f5285dfea9cc75120` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/security-loop-audit-kujo` | `2a5612c7e37c086d0aa3808127a2b88f36bc2a28` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/sourced-resource-accounting-docs` | `fbaaf244708b9384fcb321e8e6772c6764e21425` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/stable-v1.7-installation` | `59c4b794070fe1f1c4d690b1639aa4357f90809a` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/string-cache-dispatch-experiment-20261031` | `7e07db7f6818c1bd2f29f828de72b6148a1dc89c` | DEFER BEYOND 1.10 | Inconsistent workload gains/cold-cost evidence; no accepted speedup. |
| `codex/struct-field-read-costs-20261031` | `66a59e4e9abdfcacb783c80d6d4bd6fd842e774e` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/struct-generator-methods` | `59c4b794070fe1f1c4d690b1639aa4357f90809a` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/symbolized-kernels-20261031` | `2c53503ac037bbeeca87900909872a71dd205bd5` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/tribunal-v08-runtime-primitives` | `d291bacbd5e2483a07946be0e6d6a3a72bb69bbb` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/upgrade-linux-race` | `303828de5ba39343688dc309fcc35826fb880194` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/v1-8-final-prep` | `59c4b794070fe1f1c4d690b1639aa4357f90809a` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/v1-8-publication` | `573a62436462777234875d85a61f258c00d3c0b7` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/v1-release-readiness-p0` | `09c98cc941b62887cb664ae465b28a4d14062c1d` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/v1.2.3-release-docs` | `b1cec6b6c0f22e9015f5a8d3d1afc7e30b6964f7` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/vm-code-lifetime-20261101` | `fc358348d8e590d05d1110d56ee49b88424b8028` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/watchdog-kennel-receipt` | `ae9338ceab3027f9f37b116611f362b08dae6778` | INCORPORATED | Exact tip is an ancestor of main. |
| `codex/windows-process-lifetime` | `8c561cacdcb132faa310df2b81f56c27ff4537df` | INCORPORATED | Exact tip is an ancestor of main. |
| `docs/roadmap-after-runtime-hardening` | `f65ae9970b03c4ad8be890478e75b5afe3b976a9` | INCORPORATED | Exact tip is an ancestor of main. |
| `feat/core-ai-native` | `55bb2891d9c8545ca2222a39914cc8eb785654bf` | INCORPORATED | Exact tip is an ancestor of main. |
| `fix/fence-confined-output` | `59c4b794070fe1f1c4d690b1639aa4357f90809a` | INCORPORATED | Exact tip is an ancestor of main. |
| `fix/native-jsonld-hardening` | `6695bd9beac4787c9d2db768e9f61dfdb26a95a5` | INCORPORATED | Exact tip is an ancestor of main. |
| `fix/siteprobe-runtime-followups` | `5dcbfcda2e48c2fe56dd2a4ecdb2d5947e18ff59` | INCORPORATED | Exact tip is an ancestor of main. |
| `integration/runtime-hardening-wave-1` | `87fae36dd331b185d29256f74d2a92f1aefc5ea5` | INCORPORATED | Exact tip is an ancestor of main. |
| `integration/wave1-closure-import-fix` | `761c75a060c1f765933c5f9586836528871ad0f1` | INCORPORATED | Exact tip is an ancestor of main. |
| `main` | `2984bb9e531208217516f1ff42c1daeef8b7f78f` | INCORPORATED | Exact tip is an ancestor of main. |
| `release/companion-public-install` | `dbb474161f43aa0273cf0dce850ae69a836fe4db` | INCORPORATED | Exact tip is an ancestor of main. |
| `runtime/full-upvalue-closures` | `59c4b794070fe1f1c4d690b1639aa4357f90809a` | INCORPORATED | Exact tip is an ancestor of main. |
| `runtime/generator-async-spawn-audit` | `6798c10eb49a37b0a9da5b236bcf6d7cbc23c491` | INCORPORATED | Exact tip is an ancestor of main. |
| `runtime/generator-async-spawn-completion` | `cf1f1cad6ad2e66368e628e2706b786bf8393b98` | INCORPORATED | Exact tip is an ancestor of main. |
| `workflow/failure-gate-evidence-control` | `59c4b794070fe1f1c4d690b1639aa4357f90809a` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin` | `2984bb9e531208217516f1ff42c1daeef8b7f78f` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/assetworks-confined-digest` | `cc2d7dbb59a8dc05f00d629e100932f56f4062f6` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/bluepencil-confined-directory-pages` | `2a4dbe169cd889856fa6d762899f7f8892027327` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/borrowed-named-loads-20261031` | `50196d3151e7e28759bc9b80b748ed8b1f357ab1` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/checked-assignment-validation-20261101` | `568dc6127556984833617b3cd64914a2307420a2` | INCORPORATED | Patch-equivalent to 782c712d; git cherry reports minus. |
| `origin/codex/crates-http-dependency` | `23eb4ca2d2c8ef53f8dc22517aee36fd55402ad1` | DEFER BEYOND 1.10 | 23eb4ca2 registry-identity proposal requires published reviewed fork and normalized root-package verification; no publication authorized. Keep vendored HTTP safety patch. |
| `origin/codex/depth-cache-hash-20261101` | `9fb0ed705c59a8da6c6eab36e9f50c3a0722f728` | INCORPORATED | Patch-equivalent to 55e2f683; git cherry reports minus. |
| `origin/codex/dispatch-release-receipt` | `8a84fb7ce4d1dd3abaf290736939f1b7883b4d03` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/graph-attempt-accounting-docs` | `2946f2a1a76700519292db3da8004ea1a9e31f61` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/http-request-header-values` | `bdb9d4c92b01bf686bceef60adad09148aaca128` | INCORPORATED | Feature e9e5a677, evolved timeout helper, Node floor 59c4b794 present. |
| `origin/codex/http-stream-consumer-1.5` | `302bbb19ac4fb661843e4e2e3d633f7bae5d794b` | ARCHIVE / NO MERGE | Older-version backport; forward implementation d6ed9c0c in main. |
| `origin/codex/http-stream-consumer-1.6` | `92e0e0a8125da5d005c1e4f5f9a9784f436158a9` | ARCHIVE / NO MERGE | Older-version backport; forward implementation d6ed9c0c in main. |
| `origin/codex/jit-owner-reuse-20261103` | `1eea508d136a3552b6161c110f140de2692e6c8d` | DEFER BEYOND 1.10 | VM Neural adverse control; production/resource/platform gates incomplete. Keep current implementation excluded. |
| `origin/codex/json-float-roundtrip` | `b25b4edd7f882810ab29a58028ad4d8de9fcaca9` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/kennel-tool-import-isolation` | `66029f26105ed9206d88c3560790cf65b5bf1d28` | DEFER BEYOND 1.10 (fuzz proposal only) | Isolation/type fixes incorporated. 4c64b915 fuzz pinning has September toolchain/lock and absent helpers; requires current-toolchain validation. Existing fuzz failure is not acceptance evidence. |
| `origin/codex/kujo-1-8-core-quality` | `7a47d66bb8871c3a04424fbbad271d19a20e4dd3` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/loop-break-exit-20261031` | `eb299e0a699560c80d87d0d81582914ba2c6d956` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/loop-cache-invalidation-20261101` | `17c3ccd46a685ac25a53e8f0f1028e3de7524931` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/loop-cache-reuse-20261101` | `aae3919f6c161906b7dcdf8f4a8efcf44730180a` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/loop-cache-stats-20261101` | `8064895bc10293275e4cd01f29642ee5cc5685bd` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/loop-condition-admission-20261031` | `6d96c151e8b9f3d814043eebe5768ae5c61cde97` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/loop-function-fixture-20261031` | `6d96c151e8b9f3d814043eebe5768ae5c61cde97` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/loop-reuse-integration-20261101` | `348a62700179e59beccaefddc7ed4da30992d67b` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/main-scalar-load-experiment-20261031` | `036032535ad4bdf540e9c1b180c33dae3f34d517` | DEFER BEYOND 1.10 | No repeatable accepted workload improvement; main-only variant needs independent confirmation. |
| `origin/codex/msrv-lint-compat-20261031` | `2d3ee2ee6649df6074873d2ec378f25beaeb9ffe` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/native-absolute-path` | `6a0c9e3949cae246a6abbccbbe11e7c1e0bc4b28` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/native-field-fusion-20261103` | `53b190560217d66ec2b31742ec6affa49a5126ec` | DEFER BEYOND 1.10 | One passing session; independent timing, full Neural state/resource and combined-source gates missing. |
| `origin/codex/native-implementation-20261103` | `49c0d98918bd52dd43639e8866f3a7b815801f77` | DEFER BEYOND 1.10 | Local-store accepted through c27; later scalar writes lack second session/full-state/resource acceptance and have VM Quick/BWT adverse observations. Evidence/tools already retained. |
| `origin/codex/native-memory-gates-20261103` | `754453061a98fe3c994b89f1c9308326a50c544d` | INCORPORATED | Final implementation integrated via aab7328f; original candidate provenance retained. |
| `origin/codex/native-memory-platform-20261103` | `089246278e955545d98415f66b851b01803a12ce` | INCORPORATED | src/tests/Cargo tree equals accepted 160f4c9d; platform workflow changes integrated. |
| `origin/codex/native-package-primitives` | `4c408da54797ad8e22a878d328c836513bc15068` | INCORPORATED | 1778152f includes platform primitives; platform.rs identical to candidate. Do not overwrite evolved budgets. |
| `origin/codex/native-performance-campaign-20261102` | `160f4c9de13660db8c64f6871f8fb222839c4883` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/native-root-hash-experiment-20261031` | `04a9321edc5ed718404c083d73d63549ee1ac98a` | REJECT | Repeated uncached-read/Merge VM regressions; preserve evidence, do not adopt this mechanism. |
| `origin/codex/nested-dictionary-collections-20261031` | `b401dc6784b83498b08ced0e32ed079403c4844c` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/operator-readiness-docs` | `6bcd425c538b03fcefdf584affe18583eb131877` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/quoteflow-pool-fairness-v1.4` | `f1a45d3b286471aa9d7b164a6baaa3dbde34dff5` | ARCHIVE / NO MERGE | Forward FIFO fix 0c6f9f89 in main; retain old release/PDF compatibility history. |
| `origin/codex/release-consumer-checklist` | `334567c0c11f7530f9087652f982cc443a93c495` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/release-v1.7.0` | `f68750b368e6ebaf4dcaf588b4984651bb4d9670` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/release-v1.7.0-evidence` | `f1a220f2f0c2bf6d6df5732b8ef06b6b7bc7a04d` | ARCHIVE / NO MERGE | Historical registry exception/release evidence; no new runtime delta. |
| `origin/codex/sourced-resource-accounting-docs` | `ecf1c68a823878082c3946bd00d40a6b4501f7f5` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/stable-v1.7-installation` | `36aacc3255b3062b7018bf93353f58038c67419e` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/string-cache-dispatch-experiment-20261031` | `7e07db7f6818c1bd2f29f828de72b6148a1dc89c` | DEFER BEYOND 1.10 | Inconsistent workload gains/cold-cost evidence; no accepted speedup. |
| `origin/codex/struct-field-read-costs-20261031` | `66a59e4e9abdfcacb783c80d6d4bd6fd842e774e` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/struct-generator-methods` | `b176b96566b2352bfe35730611584fda566d6ab6` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/v1-8-final-prep` | `69e6e6f7226eb6e10973c4f55125772ef5d82f26` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/v1-8-publication` | `573a62436462777234875d85a61f258c00d3c0b7` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/v1-8-release-readiness` | `878a09b8194e0b9700ffd4702badd451e24abb17` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/codex/windows-process-lifetime` | `8c561cacdcb132faa310df2b81f56c27ff4537df` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/docs/roadmap-after-runtime-hardening` | `f65ae9970b03c4ad8be890478e75b5afe3b976a9` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/integration/runtime-hardening-wave-1` | `87fae36dd331b185d29256f74d2a92f1aefc5ea5` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/main` | `2984bb9e531208217516f1ff42c1daeef8b7f78f` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/release/1.6.0-reviewed` | `44af277848173664f72ca85f2a1b3b98d634ecdd` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/release/companion-public-install` | `dbb474161f43aa0273cf0dce850ae69a836fe4db` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/runtime/full-upvalue-closures` | `f7bda5b40035c9d30578f2265d1493fbfa62394d` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/runtime/generator-async-spawn-audit` | `6798c10eb49a37b0a9da5b236bcf6d7cbc23c491` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/runtime/generator-async-spawn-completion` | `cf1f1cad6ad2e66368e628e2706b786bf8393b98` | INCORPORATED | Exact tip is an ancestor of main. |
| `origin/workflow/failure-gate-evidence-control` | `676cbf4ea143f5d661f0087b00118150adfd2270` | INCORPORATED | Exact tip is an ancestor of main. |
