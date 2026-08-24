# Kujo ecosystem hardening review — 2026-08-24

## Executive summary

The highest-leverage flagship is the Kujo → AI SDK → Agents SDK → Dispatch
control-plane stack. Its local-first fixture contracts already compose well;
the missing product value is one evidence-producing golden path, not another
standalone showcase. Source and CMS remain valuable fallback surfaces, but
both retain external certification/governance blockers.

This review treats documentation as evidence to verify. It distinguishes local
implementation work, external prerequisites, and stale planning entries.

## Prioritized execution table

| Priority | Repository | Issue/opportunity | Evidence | Classification | Action | Dependency | Effort | Confidence |
|---|---|---|---|---|---|---|---|---|
| P0 | Kujo/TotalRecall | VM fixture output emitted false undefined-builtin warnings | `kujo/src/type_checker.rs`, `totalrecall/tests/*.kujo` | Locally solvable; fixed in this change | Register runtime assertion/time builtins in the checker and retain regression coverage | Kujo test/build | S | High |
| P0 | TotalRecall | Fixture gate snapshot mismatch | `totalrecall/tests/*.out`, `kujo test --runtime vm -v` | Active local verification item | Rerun after the Kujo fix; update snapshots only if output intent is unchanged and reviewable | P0 Kujo fix | S | High |
| P0 | Workcell | Podman/Linux release evidence absent | `workcell/docs/launch-checklist.md` | External | Run rootless Podman doctor, OCI, integration, load, egress, cleanup gates on supported Linux | Linux + Podman | M | High |
| P0 | Workcell/Source/CMS | Hosted CI or branch rules unavailable | `workcell/docs/launch-checklist.md`, `source/EXTERNAL-BLOCKERS.md`, `cms/docs/enterprise-production-readiness-plan.md` | External | Restore hosted runner/billing and apply GitHub rulesets; retain exact receipts | Admin/billing | M | High |
| P0 | Source | Documented credential audit findings | `source/EXTERNAL-BLOCKERS.md`, `source/src/scan.js` | Requires external verification; not reproduced locally | Run an independent security scan and triage its exported report | Scanner host | M | Medium |
| P0 | TotalRecall | Path confinement and output redaction backlog | `totalrecall/docs/NEXT_SESSION_ENTERPRISE_BACKLOG.md`, `src/core/common.kujo`, `src/core/redaction.kujo`, filesystem tests | Mostly already fixed; coverage remains | Add one end-to-end redaction regression spanning stdout, report, audit, and JSON errors | Kujo runtime + harness | M | High |
| P0 | SearchBridge/Intake/Relay | Live/provider/release proof | respective next-session backlogs | External | Obtain approved credentials, provider accounts, release authorization, and hosted CI | Human/operator action | M-L | High |
| P1 | Ecosystem | Golden-path orchestration | `kujo/src/ecosystem_golden_path.rs`, `kujo ecosystem golden-path`, existing examples | Implemented locally; stage health still varies | Use the runner to produce one isolated evidence bundle; investigate blocked/failed stages before claiming a clean run | Dispatch/host availability | M | High |
| P1 | Roadmaps | Status drift and stale backlog entries | `kujo/ROADMAP.md`, `ssg/ROADMAP.md`, `totalrecall/docs/NEXT_SESSION_ENTERPRISE_BACKLOG.md` | Stale/superseded entries mixed with active work | Reconcile status against code, tests, and recent commits | Maintainer review | M | Medium |
| P2 | Fence/Kennel/SSG | Post-release enhancement queues | respective roadmap/backlog files | Post-release | Keep queued; do not displace release/security gates | v1 release evidence | M-L | High |

## Security triage

| Finding | Evidence checked | Classification | Rationale/next action |
|---|---|---|---|
| Source embedded private key / hardcoded credential audit | `source/src/scan.js`, credential-scan tests, repository search; no PEM/private-key literal reproduced | Requires external verification | The documented audit claim cannot be accepted or rejected without the missing independent report. Re-run the scanner and preserve the report. |
| TotalRecall destination traversal | `src/core/common.kujo:safe_relative_note_id`, all filesystem destinations, `tests/filesystem_destinations_test.kujo` | Already fixed locally | Existing IDs reject separators, `..`, and unsafe characters; traversal regression cases exist. |
| TotalRecall sensitive output leakage | `src/core/redaction.kujo`, `main.kujo`, config/encrypted-state harnesses | Partially fixed | Central masking is applied to logs, reports, audit events, and errors, but full end-to-end coverage is still queued. |
| AI Chat credential/auth/tool boundaries | `ai-chat/SETUP_AND_INSTALL.md`, `README.md` | No local finding reproduced | Encryption, API auth, loopback Watchdog, bounded local tools, browser restrictions, and approval boundaries are documented; live-provider validation remains external. |
| Redact completeness | `redact/SECURITY.md`, `redact/docs/security.md` | Product limitation | Redact explicitly does not guarantee perfect sensitive-data detection; treat it as review assistance, not certification. |
| Workcell isolation/secret/egress boundaries | `workcell/README.md`, `docs/security-model.md`, `docs/known-limitations.md` | Valid bounded limitation | Engine/host kernel, image governance, key custody, retention, firewall/proxy, and compliance remain outside Workcell. |
| Watchdog auth/redaction | `watchdog/src/dashboard_server.kujo`, shared helpers, fixture tests | No local finding reproduced | Token headers and sanitized telemetry paths exist; live provider proof is still an external prerequisite. |

## Golden-path gap analysis

Existing evidence is strong but distributed:

- Kujo and AI SDK provide VM execution, replay/fixture mode, normalized AI
  envelopes, schema checks, budgets, and redaction.
- Agents SDK provides no-network harnesses, agent runners, traces, artifacts,
  and example smoke tests.
- Dispatch provides offline workflows, approvals, resume, signed bundles,
  traces, and JSON state.
- Workcell provides controlled container execution, receipts, manifests,
  cleanup, artifact policy, and egress declarations.
- Watchdog provides telemetry/proxy boundaries but requires separate live proof.
- Eval provides deterministic checks, reports, checksums, and manifest verify.
- ShipCheck provides a local release gate with JSON/Markdown output.
- RunLedger and CaseFile provide local run and failure evidence contracts.
- Concord and Fence provide drift and architecture-boundary checks; Tribunal
  provides sealed decision evidence when human review is needed.

The integration is now implemented as `kujo ecosystem golden-path`. It uses
existing entrypoints, writes one isolated output root, emits per-stage JSON
results plus stdout/stderr artifacts, hashes the evidence bundle, and labels
fixture versus local-real evidence. Missing infrastructure is represented as
`blocked`; the command fails closed unless `--allow-blocked` is explicitly
provided. It does not manufacture platform-CI or external-live evidence.

## Flagship recommendation

Primary: the Kujo/AI SDK/Agents SDK/Dispatch control-plane stack. It has the
highest ecosystem leverage, strongest deterministic foundations, and clearest
public technical story. The remaining work is integration/evidence packaging,
not broad product invention.

Fallback: AI Chat. It has direct user value and a compelling local showcase,
but its security/release posture depends on encrypted secrets, app auth,
Watchdog integration, provider configuration, and a larger runtime surface.

Source and CMS are credible later product surfaces, but Source still lists
hosted CI, independent security, multi-region, capacity, and compliance gaps;
CMS still lists branch protection as an open launch gate.

## External blockers requiring human action

- Hosted GitHub Actions runner allocation/billing and exact approved-commit
  receipts.
- GitHub branch protection/ruleset administration for CMS.
- Rootless Podman/Linux host validation for Workcell.
- Independent Source security assessment and report export.
- Production topology multi-region, capacity, backup/restore, retention,
  access-review, incident-response, and compliance evidence.
- Provider credentials/accounts and release-owner authorization for SearchBridge,
  Intake, and Relay live proof.
- Production deployment, signing-key custody, image governance, and firewall/
  proxy/egress controls.

## 30/60/90-day sequence

### 0–30 days

1. Land and verify the Kujo builtin-checker fix.
2. Regenerate TotalRecall's intentionally ignored local snapshots with the
   pinned Kujo binary, then re-run the VM/dual fixture gates.
3. Run `kujo ecosystem golden-path --allow-blocked --json`; preserve the
   isolated bundle and investigate Dispatch/host blockers rather than masking
   them.
4. Add the missing TotalRecall end-to-end redaction harness.

### 31–60 days

1. Add a dedicated CaseFile adapter that preserves its repository-scoped
   output contract while writing under the isolated evidence root; RunLedger
   integration is now implemented.
2. Persist RunLedger/CaseFile references and Workcell receipts under the
   isolated evidence root.
3. Add Concord/Fence checks to the release evidence summary.
4. Reconcile roadmap status and mark completed/stale items explicitly.

### 61–90 days

1. Obtain hosted CI, Podman/Linux, branch-protection, provider, and security
   evidence.
2. Run capacity, resilience, backup/restore, retention, and compliance drills.
3. Publish a fixture-only public showcase first; add live-provider claims only
   after external receipts exist.

## Verification record

- `cargo fmt --check` passed.
- `cargo test runtime_test_builtins_are_warning_free --lib` passed (1 test).
- `cargo test type_checker::tests --lib` passed (24 tests).
- `cargo test --test optional_typing_v1_contract` passed.
- AI SDK fixture passed with `Mode: fixture` and normalized JSON output.
- Agents SDK smoke runner passed with all seven example statuses reported.
- Eval release-gate suite passed 3/3 checks with artifact checksums.
- ShipCheck gate passed 16/16 checks with zero warnings/errors.
- `kujo ecosystem golden-path --allow-blocked --json` produced an isolated
  bundle with per-stage results, stdout/stderr artifacts, SHA-256 manifest, and
  evidence handoff; AI SDK, Agents SDK, Eval, and ShipCheck passed while
  Dispatch, Workcell, and Watchdog were explicitly blocked in this host.
- The same command without `--allow-blocked` returned non-zero, proving the
   fail-closed policy for blocked/failed stages.
- RunLedger start/finish was recorded under the isolated bundle with a
  partial status reflecting blocked stages; CaseFile currently has an explicit
  local path/hash reference and is not falsely presented as an external case.
- TotalRecall snapshots are generated and intentionally ignored by that
  repository's `.gitignore`; after regeneration, `kujo test --runtime vm`
  passed 13/13 with `vm_primary=13`.
- TotalRecall's dual gate passed 13/13 after snapshot regeneration with
  `vm_primary=13` and no interpreter fallback.
- Dispatch did not complete within a bounded 15-second offline run in either
  VM or interpreter mode; the captured output is an unresolved local blocker,
  not evidence of a passing workflow stage.
- No live credentials, hosted services, or production infrastructure were
  used.
