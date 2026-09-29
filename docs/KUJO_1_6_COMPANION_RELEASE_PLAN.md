# Kujo 1.6 companion release triage

Original triage reviewed September 28, 2026; the historical queue below was not
publication authorization. The user subsequently authorized official releases. Kujo 1.6 itself remains released. No companion is a prerequisite
for running ordinary Kujo programs. The new companion features do require
separate distributions before normal version-based installation can use them.

[Machine-readable assessment and exact revisions](evidence/kujo-1.6-companion-audit/assessment.json)
retain official-release identities, current fetched main commits, scoped tests,
hosted job evidence, and hashed local logs.

## First batch completed — official releases

The user authorized official publication and explicitly waived the AI SDK live-provider
check for **1.1.1 only**, to run on their next pass. This is not live-provider
validation or a waiver for later releases.

| Release | Public delivery and final verification |
| --- | --- |
| [RunLedger 1.2.0](https://github.com/kujolang/runledger/releases/tag/v1.2.0) | GitHub and Kennel published. Hosted Linux/macOS gates pass; fresh public tool install passes 81 module tests, CLI/concurrency and measurement-reference tests. |
| [AI SDK 1.1.1](https://github.com/kujolang/ai-sdk/releases/tag/v1.1.1) | GitHub and Kennel published. Full 150-test offline release gate and benchmarks pass from the public package; hosted supply-chain gate passes. Real-provider check explicitly skipped. |
| [Agents SDK 1.1.2](https://github.com/kujolang/agents-sdk/releases/tag/v1.1.2) | GitHub and Kennel published. Exact-source hosted offline/context/ratchet gate passes; fresh public install passes 41/41 and VM/interpreter consumer imports. |

Public installation exposed a Kennel publisher filter that removed the legitimate
`src/agents/artifacts/store.kujo` module. The first corrective publication also
exposed a stale source checkout pin inside the reusable publisher workflow. Both
pins and the filter are corrected. The **1.1.2** public archive matches the
pre-publication archive exactly and contains the module. Incomplete 1.1.0/1.1.1
registry archives remain immutable; their GitHub release notices direct users to
1.1.2. The released Kennel 1.1.0 client works with the corrected packages.

[Final receipt, exact source/tag identities, public archive hashes and retained logs](evidence/kujo-1.6-companion-candidates/official-releases/receipt.json)
supersede the pending statuses in the historical preparation/execution snapshots
below. Wave C beta and Wave D alpha stay experimental. Separate participant SDK
packages remain private and unpublished. Other tools in the original queue are
not certified or released by this first batch.

## Second batch completed — Workcell, Ability and MCP 1.2.0

All three are official, non-draft GitHub releases and published Kennel packages:

| Release | Reviewed source | Final verification |
| --- | --- | --- |
| [Workcell 1.2.0](https://github.com/kujolang/workcell/releases/tag/v1.2.0) | `4ec4227f8190e05a329c2ee2975d77fce7872313` | Full Linux/macOS release gates, supported Docker/Podman lifecycle, real Git/process assurance, dependency audit and public installation pass. |
| [Ability 1.2.0](https://github.com/kujolang/ability/releases/tag/v1.2.0) | `2dc7d4e9eb25e987544ac48793cd11d1999c45e0` | Full release/package and application-gateway assurance gates, public archive and fresh Kennel installation pass. |
| [MCP 1.2.0](https://github.com/kujolang/mcp/releases/tag/v1.2.0) | `427a8b9ecfaee97cd66c6b35137bd1dc5fb16b54` | Full framework, controlled STDIO, framing/host bridge and fresh public installation pass; Ability dependency pins its released source. |

The release scope uses Kujo 1.6.0. Workcell's reviewed upstream E2B dependency
update resolves its audit/SBOM issue. MCP's fresh local host evidence does not
promote remote authentication or renew historical remote/editor certification.
All registry archive files match the exact tagged sources. The public install
matrix uses the released Kennel client and fresh consumer lockfiles.

The ecosystem website, documentation and read-only MCP catalog now describe
these releases. Wave C beta and Wave D alpha remain experimental; separate
participant SDKs remain unpublished. Dispatch was not part of this second batch;
its subsequent official release is recorded below.

[Final second-batch receipt and retained verification evidence](evidence/kujo-1.6-companion-candidates/second-cohort/receipt.json)
supersede the pending statuses for these three tools in the historical audit.

## Controller release completed — Dispatch 1.3.0

[Dispatch 1.3.0](https://github.com/kujolang/dispatch/releases/tag/v1.3.0) is an
official GitHub release and Kennel package at
`7558d0b2157449a484646d9a4b0d069a70c36f85`. The final Linux/macOS candidate and
tagged-release pipelines passed, including upgrade/backup rollback and fresh
installed workflows. Both public archives match all 885 tagged source files.
The released Kennel client installs the exact package and pinned AI SDK dependency;
its filesystem and SQLite workflow probes pass without source-checkout fallback.

This completes the requested controller cohort with Kujo 1.6.0, AI SDK 1.1.1,
Agents SDK 1.1.2 and the Workcell/Ability/MCP 1.2.0 adapter family. The user also
authorized including concurrently developed selected-effect work after checking
that it did not interfere; the combined candidate passed focused tests, scoped
security review and the full gates. Review callback hardening preserves configured
tool policy, scopes approval to its step and serializes duplicate legacy decisions.

The website, docs release inventory, MCP discovery catalog and Kennel now identify
Dispatch 1.3.0. Wave C beta and Wave D alpha remain experimental. The separate
selected-effect operator API does not grant parent replay or automatic remainder
execution. Participant SDKs stay unpublished; agent rehearsal is not human usability
validation, and localhost transport tests are not remote-provider certification.

[Final controller receipt, public hashes, pinned install lockfile and live-site evidence](evidence/kujo-1.6-companion-candidates/dispatch/receipt.json)
supersede Dispatch's pending status in the historical audit. No runtime or companion
version was changed by this post-publication evidence update.

## Watchdog and Kennel releases completed

[Watchdog 1.2.0](https://github.com/kujolang/watchdog/releases/tag/v1.2.0)
and [Kennel 1.1.1](https://github.com/kujolang/kennel/releases/tag/v1.1.1)
are official GitHub releases and immutable Kennel registry packages. Watchdog
ships verified Kujo 1.6 runtime measurement observations and RunLedger correlation;
Kennel ships binary-safe bootstrap and registry/source preservation fixes.

Both full local and hosted gates pass. Kennel retains Kujo 1.4 compatibility,
while current companion production installs run on 1.6. Fresh public installs
pass for all six packages in the companion matrix, and Kennel bootstrap passes
on macOS and Linux. Source archives, registry archives and provenance match the
tagged sources. Website, documentation and MCP discovery metadata are updated.
Watchdog remains observational; experimental Wave C/D status is unchanged.

[Release identities, public hashes, gate URLs and deployment receipt](evidence/kujo-1.6-companion-candidates/watchdog-kennel/receipt.json)
supersede the pending Watchdog/Kennel entries in the historical triage below.

## Historical decision and release order

“Prepare now” means finish the listed gates, then review an exact candidate. It
never means release the current tip merely because it has many commits.

| Tool | Latest GitHub Release / commits ahead | Queue | Why / exit condition |
| --- | --- | --- | --- |
| AI SDK | v1.0.0 / 44 | Prepare now: dependency foundation | Unreleased credential-error redaction, transport containment, wrapper cleanup and validation fixes deserve distribution independent of Wave C/D. Repair unreachable CI runtime pin; reconcile existing v1.1.0 tag before selecting a new immutable version; run full offline and documented provider release gates. |
| Agents SDK | v1.0.0 / 51 | Prepare now: corrective + integration release | Released runner suite has two failures on both 1.5 and 1.6; current runner passes 25/25. Includes policy/persistence hardening, context work, telemetry and controlled Ability tools. Resolve failing hosted offline suites and reconcile minimum-runtime/Ability dependency claims. |
| Dispatch | v1.2.0 / 111 | Prepare now: coordinated control release, publish after dependency closure | Durable locks, review/checkpoints, failure policy and replay controls are significant user-facing work. Ship stable control improvements with assurance/interoperability explicitly experimental. Reconcile metadata, installer/dependency closure, upgrade/rollback and final hosted Linux/macOS gates before tagging. |
| Workcell | v1.1.0 / 17 | Prepare now alongside Dispatch | Failure preservation and execution evidence support the control workflow; Git assurance/process participant stays experimental. Resolve adapter drift integrity failure; reconcile runtime compatibility and complete supported Docker/Podman release gates. |
| Ability | v1.1.0 / 12 | Prepare now alongside controlled participants | Public definition digest, profiles, application assurance and bounded HTTP participant support are needed for the new integrations. Resolve failing package gate, validate baseline and experimental runtime requirements separately, and keep assurance opt-in. |
| MCP | v1.1.1 / 17 | Coordinated cohort after Ability | New controlled STDIO handoff and framing/duplicate protections require a tool release to reach users. Current CI is green, but not final-cohort certification. Reconcile Ability dependency/packaging and rerun standalone + controlled install tests. |
| RunLedger | v1.1.0 / 12 | Near-term independent maintenance release | Correct committed-file evidence, unusual Git path handling, private ledgers, report escaping and strict verification should not wait for the large Dispatch cohort. Run full final release/installer gates and consolidate changelog. |
| Watchdog | v1.1.0 / 10 | Can follow the first cohort | Latest release is recent; current API route smoke works on 1.6. Main adds measurement references, pricing and CI/test fixes. Release when distributing the measurement feature, not merely to change a runtime label. |
| Kennel | v1.1.0 / 14 | Conditional dependency; otherwise can follow | Released deterministic/stale-lockfile probes pass on 1.6. Many changes are registry delivery/docs, already deployed separately. If new packages require the newer Git-dependency metadata path, verify a released-client install and ship the necessary Kennel/publisher fix first. Never replace existing immutable registry packages. |
| SSG | v1.0.0 / 23 | Defer from the 1.6 control cohort | Ability preview/WebMCP and generator fixes form a separate website-tool release. Runtime help smoke passes; current full CI is red. Resolve its own gate and rendering issues in that release, not as a Kujo-runtime prerequisite. |

AI SDK has an important identity wrinkle: **v1.1.0 is already a Git tag**, and
main is eight commits ahead of it, but GitHub has no Release for that tag and
the reviewed Kennel registry snapshot contains 1.0.0. The previous “44 commits”
count is correct for latest official GitHub Release, not latest existing tag.
Do not overwrite or move v1.1.0 to resolve this. Verify channels and choose a new
version or publish only the already-tagged historical bytes with accurate scope.

Suggested execution batches:

1. AI SDK and Agents SDK corrective candidate preparation. RunLedger may proceed
   independently; it does not need to wait for Wave D packaging.
2. Workcell and Ability candidates, then MCP and Dispatch against one explicit
   tested dependency set. Dispatch is the final controller integration gate.
3. Watchdog measurement release; Kennel if the installed-package dependency
   tests require it. SSG gets a separate release review.

A safe short delay for a blocked candidate is preferable to publishing a failing
one. Reassess priority when a deployed-user issue or dependency requires it;
commit count is neither severity nor proof of readiness.

## Historical first-batch preparation snapshot

The user approved starting AI SDK, Agents SDK and RunLedger candidate preparation.
Draft candidates are pushed separately:

- [AI SDK 1.1.1](https://github.com/kujolang/ai-sdk/pull/2): immutable published runtime source pins replace inaccessible CI revisions. Local release gate passes 150 aggregate tests. The required real-provider release smoke remains blocked by missing configured credentials; no skip has been authorized.
- [Agents SDK 1.1.0](https://github.com/kujolang/agents-sdk/pull/2): CI now uses Kujo 1.6.0. Offline gate passes 41/41; context contracts, 20 paired repetitions, 14 VM/interpreter executions and token ratchet pass. The existing Ability dependency is preserved.
- [RunLedger 1.2.0](https://github.com/kujolang/runledger/pull/1): canonical suite passes 81 module tests, CLI integration and measurement references. Explicit CommonJS test encoding removes dependence on parent Node metadata; Linux/macOS functional CI is added.

All three local source-archive gates passed outside Git checkouts. This is not
Kennel registry installation certification. Hosted checks were still queued or
running at this snapshot; inspect their final results before release. Candidates
remain drafts, without tags or publication. Runtime minimum claims and final
package/dependency installation still require release review.

[Candidate commits and hashed verification logs](evidence/kujo-1.6-companion-candidates/candidates.json)
retain this preparation evidence. Historical audit findings below describe the
original main revisions and are not overwritten by candidate results.

## Historical execution snapshot before runner completion

The user explicitly requested official releases for AI SDK, Agents SDK and
RunLedger, rather than stopping at candidate preparation. All three PRs are
now ready for review (not draft). Release documentation/install instructions are
updated, RunLedger has a native Kennel manifest, and the supported full-release
runtime is Kujo 1.6.0. Agents SDK's manifest now uses the canonical `[kujo]` table.
AI SDK CI uses published checksum-verified runtimes rather than rebuilding them.

Local final and installed-package gates pass. AI SDK and RunLedger ShipCheck
metadata gates pass 16/16; Agents SDK passes with two lint/format-command
advisories. RunLedger lint has one advisory for parsing a fixed JSON NUL literal.
No affected runtime implementation was changed in this release pass.

Publication is still pending, not declined or awaiting general permission:
GitHub runners remain unassigned on the final jobs despite bounded retries,
and AI SDK's release policy requires a provider secret that is not configured.
Its full release workflow was dispatched with skipping disabled. The current
sources, logs and workflow IDs are in
[the execution record](evidence/kujo-1.6-companion-candidates/release-execution.json).
After these checks pass, merge/tag the tested sources, create official GitHub
Releases, run the central Kennel reconciliation and verify public fresh installs.
Do not stop again at candidate preparation or request release permission again.

### Historical first publication snapshot

[Agents SDK 1.1.0](https://github.com/kujolang/agents-sdk/releases/tag/v1.1.0)
is now an official, non-draft GitHub Release (ID 398678883), tagged at the exact
tested source `eaae7c4feea7c2a7f7988f01340363547e01bfa7`. Its hosted gate passed
and its public source archive matches the tested manifest. Kennel reconciliation
run 36493279767 is queued; public registry delivery is not yet certified.

RunLedger's final hosted Linux/macOS gate remains queued (fresh manual run
36493688172). AI SDK's hosted contracts pass; a missing SBOM output directory
was fixed and full release workflow 36493540693 was dispatched with provider
skipping disabled. The provider credential remains required. These are ongoing
official-release tasks, not requests for another general release approval.

## What was actually tested

Tests used isolated exports of the official release tags and the published
macOS x64 Kujo 1.6.0 binary, SHA-256
`930e0da1fec2562f6990d1226a479330640c8c78eb4c36c148cee3f950b29a47`.
They are source compatibility probes, not cross-platform or clean-install
certification. The executable is from release source
`44af277848173664f72ca85f2a1b3b98d634ecdd`.

| Released tool | Kujo 1.6 result / scope |
| --- | --- |
| Dispatch 1.2 | Five focused suites passed:10+3+31+1+8 tests, plus first shard 5/5. Aggregate stopped at the explicit 240-second audit budget during shard 2/24. **Incomplete**, not a runtime failure or full-gate pass. |
| Workcell 1.1 | Core 213/213 and workspace 28/28 passed after supplying the canonical secret-test environment. Initial 13 failures from omitted fixture variables are retained and superseded by the correct run. No Docker/Podman certification performed. |
| Ability 1.1 | `tests/run_tests.sh` passed: definition/runtime contracts, cross-language SDK conformance, registry/devkit tests and syntax check. Consumer/Fence release gate remains separate. |
| Agents SDK 1.0 | Runner 23/25; tool registry 16/16, approval 19/19 and result/event 2/2. Same two runner failures on 1.5: missing `stream` map key and citation expectation mismatch. Current main runner 25/25. These are existing release test/behavior discrepancies; this audit does not classify both as product bugs or claim full current SDK certification. |
| MCP 1.1.1 | Canonical unit harness passed after installing its exact locked Ability commit `4aa354da8d02b027c459f692f69b523f96e97056`. Initial missing-module failure was audit setup, not runtime incompatibility. |
| Watchdog 1.1 | Real local API-route suite passed. Full regression matrix not rerun. |
| RunLedger 1.1 | `tests/run.sh` module and CLI integration suites passed. |
| Kennel 1.1 | Deterministic lockfile and stale-lockfile suites passed. New-package install path not yet certified. |
| AI SDK 1.0 | Core contract and security-redaction suites passed. This does not cover later error-field redaction fixes or live providers. |
| SSG 1.0 | CLI help smoke passed; no new full build/release certification. |

Current Dispatch's directory-durability runtime contract passes on 1.6 and fails
on 1.5 with missing `sync_directory_beneath`. Thus **new Dispatch needs the newer
runtime capability**, not the reverse. Its metadata still declares a 1.5 minimum
plus source-pin qualifications; release preparation must state a coherent
supported runtime requirement.

ShipCheck `gate --dir <dispatch> --format json` exited 0: 16/16 checks, zero
warnings/errors. This is metadata validation only, not a substitute for the
hosted failures below or final product gates.

## Current-tip hosted evidence and release blockers

Fresh GitHub API observations, not inferred from old green receipts:

- **Dispatch:** [CI36461672966](https://github.com/kujolang/dispatch/actions/runs/36461672966)
  fails on Linux and macOS at “Verify immutable dependency closure and installer
  parser.” The retrieved log reports exit 1 without an identified subcommand;
  root cause remains to be isolated. README and active checklist still name
  runtime `0d7189b…` while current refs/workflow use final `44af277…`. Package
  notes/dependency and the historical 1.3 closure describe older revisions.
  Preserve historical receipts; add/reconcile the final release closure instead
  of silently rewriting evidence. Existing checklist also leaves target staging,
  approved provider evidence, final security review and release installs open;
  later offline assurance proofs must be mapped to those boxes explicitly.
- **AI SDK:** [CI33941820537](https://github.com/kujolang/ai-sdk/actions/runs/33941820537)
  cannot fetch runtime `7819d1475e909ec88f510c9927445791ea928836`
  (`upload-pack: not our ref`). Its compatibility matrix is also red. This is
  build/provenance infrastructure evidence, not proof of a 1.6 runtime defect.
- **Agents SDK:** [CI36337338964](https://github.com/kujolang/agents-sdk/actions/runs/36337338964)
  fails offline context and controlled-Ability suites; workflow selects Kujo 1.3.1.
  Reproduce using the advertised minimum and 1.6, then correct implementation or
  supported-runtime claims. Do not assume that just raising the CI pin fixes it.
- **Ability:** [CI36345280038](https://github.com/kujolang/ability/actions/runs/36345280038)
  package verification fails with runtime error `definition` on its 1.2.2 lane;
  application-assurance lane succeeds on a separate source runtime. Reconcile
  both lanes and their published compatibility statements.
- **Workcell:** main CI passed, but [adapter drift36459625607](https://github.com/kujolang/workcell/actions/runs/36459625607)
  fails with `stale dependency integrity metadata`. Review the changed dependency
  against its commitment; do not blindly regenerate the hash. Release docs still
  pin 1.2.1, while source-runtime override example says 1.5.0.
- **SSG:** [CI35783948149](https://github.com/kujolang/ssg/actions/runs/35783948149)
  failed its repository checks; exact final failing assertion is not established
  by this audit. A help smoke cannot clear that blocker.
- **MCP, Watchdog, Kennel:** observed current-tip functional CI succeeded.
  **RunLedger:** observed current-tip artifact guard succeeded; no functional
  hosted run was returned for that tip. These observations do not certify a
  future versioned package or a new cross-tool closure.

## Dependency and packaging boundary

The current Dispatch validation cohort explicitly pins AI SDK, Agents SDK,
Workcell, Ability and MCP main commits (all listed in the JSON assessment).
That is stronger evidence than an unpinned checkout, but differs from its old
installer/package dependency closure. A source cohort does not prove released
packages install the same files.

Agents SDK and MCP still declare the older Ability commit `4aa354d…` for their
base package paths. Controlled application fixtures use newer owner behavior.
Determine which files are bundled, host-supplied, optional or required, and test
both standalone and controlled installs. Do not upgrade every dependency blindly
or make experimental integrations mandatory for ordinary usage.

For each candidate, before publication:

1. Choose a scoped changelog and immutable version; preserve alpha/beta contract
   identifiers independently of product versions. No public participant SDK
   publication is part of this queue.
2. Reconcile CLI/manifest/version/README and runtime minimum versus tested pin.
3. Close the named red gates with exact-source evidence, then run the complete
   canonical gate on supported release platforms. Include required real
   provider/container evidence only for claims actually being released.
4. Install the candidate through its real distribution path in a clean consumer
   with the reviewed dependency set; run legacy upgrade/rollback tests where
   durable state or lock protocols change.
5. Re-run the final Dispatch integration cohort against released Kujo 1.6 and
   candidate companion artifacts, preserving standalone regressions and all
   experimental boundaries.
6. Produce source/archive/checksum/provenance/registry receipts. Obtain exact
   publication authorization; never move an existing tag or replace immutable
   package bytes.

No runtime behavior, companion source, version, tag or package was changed by
this audit. It is safe to start release preparation now; it is not safe to call
all current tips ready for publication.
