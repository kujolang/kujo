# 1.6 release truth and pin audit

Baseline: Kujo `50d68997ec7cbd93440776ff6ba7d12d802d991d`.
No global text replacement was used. Version numbers in dependency locks and
numeric examples are not Kujo identity. Historical logs/evidence remain immutable.

| Surface | Classification / disposition |
|---|---|
| Cargo.toml; kujolang stanza in Cargo.lock | Current identity: 1.6.0; lock updated by Cargo, no dependency upgrade |
| CLI/REPL | Derived from CARGO_PKG_VERSION; new automated identity contract verifies CLI |
| npm workspace/runtime/five platform manifests | Current package-source identity: 1.6.0; exact native optional dependency versions; not published |
| release/kujo-1.6.0-rc.json | Candidate metadata, archive naming and intended tag; publication false |
| README source badge/statement; ROADMAP crate version | Current source candidate 1.6.0; published stable explicitly separate |
| README/architecture/spec/scope/agent guide stable statements | Published stable 1.5.0 remains true; candidate status explicitly added |
| INSTALLATION, install.sh, setup action/default/example, BUILD_AN_AGENT, INSTALL_MATRIX, RELEASE_BINARIES download examples | Published-artifact selection remains 1.5.0: changing before publication would break installation |
| .github/scripts/check-release-state.sh | Corrected stale assumption that source equals latest stable; validates both separately |
| CLI/LSP/language contract versions 1.0.0 | Compatibility identities; unchanged, not runtime versions |
| STANDARD_LIBRARY since-1.5 labels | Historical introduction versions, retained |
| RUNTIME_MEASUREMENTS and runtime/control audit documents | Post-1.5 source work / dated evidence, retained; 1.6 release notes describe current inclusion |
| NEXT_PHASE_ARCHITECTURE and version-consistency audits | Dated baseline/published artifact evidence; do not rewrite their pins or claims as new test results |
| Old release checklists, notes, generated evidence and benchmark logs | Historical, retained |
| Cargo.lock non-kujolang versions; vendor/npm dependencies; numeric 1.5 literals | Unrelated package versions/data; unchanged |
| CHANGELOG pre-1.5 history | Retained byte-for-byte; duplicate Unreleased/prototype chronology consolidated into unpublished 1.6 section |
| CONCURRENCY stale source-version phrase | Corrected to current 1.6 source candidate |

## Downstream distinctions

Dispatch `release/dispatch-v1.3.0.refs` is an immutable older candidate closure,
not the current source-runtime test pin or a published-runtime claim. Preserve it
for historical installer/compatibility tests. Current CI KUJO_RUNTIME_REF and a
separate `release/kujo-1.6.0-rc.refs` select the exact RC source; current checkout
assertions compare with those pins, never the historical manifest. A candidate
receipt records binary SHA-256 and test provenance.

Workcell RUNTIME_VERSION/docker/release checks pin its historically released
Kujo 1.2.1. Its minimum_version is a compatibility floor. Neither is a current-main
pin. WORKCELL_TEST_KUJO_VERSION is an exact **test override**, compared to the
provided executable's --version. Use 1.6.0 for this candidate; preserve release
pins/minimum. Do not claim that version text alone proves source identity.

Ability, MCP, Agents SDK and AI SDK historical released-runtime pins and test
fixture commitments remain unchanged. The 1.6 manifest records exact current
supporting checkout SHAs used by the Dispatch gate. No companion release is
implicitly cut by changing Kujo. Existing user-owned Agents SDK maintenance files
are preserved and are not part of this candidate.

## Status/claims audit

Closure/generator/task descriptions defer to completed runtime contracts, not
old completion-plan headings. Historical plans remain historical. New release
notes consolidate completed scope without changing semantics. Dispatch owns
review/replay; Watchdog/RunLedger are observational. Wave C beta remains opt-in
required/deny with alpha retained; Wave D/SDK remain alpha/unpublished/local.
Human usability remains unperformed post-release validation. Performance claims
retain measured overhead and attribution limits. See RELEASE_NOTES_1_6.md.

| Supporting CI selection | Meaning / disposition |
|---|---|
| Ability stable job v1.2.2; assurance job 5d72aab | Existing baseline cohorts retained; new RC family evidence comes from exact Dispatch-controlled run, not a claim those hosted jobs ran on RC |
| MCP stable job v1.2.2 | Historical compatibility baseline retained |
| Agents SDK stable job v1.3.1 | Historical compatibility baseline retained; current Wave D source is pinned separately in Dispatch |
| AI SDK 7819d147 CI/release-validation and 00d0bb9/595ab87 matrix | Independent exact baseline source cohorts retained; RC manifest records current SDK checkout tested downstream |
| GitHub action SHAs | Action implementation identity, not Kujo runtime version; unchanged |
