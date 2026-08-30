# Kujo Tool Artifact Ignore Inventory

Status: research snapshot
Last updated: 2026-08-30

This inventory captures local files and directories created by Kujo and adjacent tooling so each repository can share one ignore block. The reusable block is tracked at `config/kujo-tool-artifacts.gitignore`.

## Research Method

- Scanned local repositories under `/Users/robertdevore/2026/Kujolang/kujo-repos` for generated artifact directories already present in working trees.
- Cross-checked tool source, help text, and docs for default output roots, `--out`, `--output`, `--output-dir`, `--artifacts`, `--ledger`, `--baseline`, and direct write paths.
- Did not treat intentionally tracked examples or fixtures as ignore requirements unless the same path is also used as local runtime output.

## Core Local Artifact Roots

| Tool | Default/generated paths | Notes |
| --- | --- | --- |
| Loop Engineering | `.loop-engineering/`, `SUMMARY.md`, `blockers.md`, `ledger.tsv`, `loop.yml`, `checklist.tsv`, `evidence/`, `iterations/<n>/` | Repo-local loop state and verification evidence. |
| RunLedger | `.runledger/`, `.runledger/runs/`, `RUNLEDGER_REPORT.md`, `runledger-entry.json` | `--ledger` can redirect the ledger; report path is caller-selected. |
| CaseFile | `.casefile/<case-id>/case.{md,json}`, `.casefile-agency-loop/<case-id>/case.md`, `command.txt`, `environment.json`, `git-*.txt`, `stdout.log`, `stderr.log`, `combined.log`, `reproduction.md`, `handoff.md` | `casefile.toml` is created by `init`; keep it tracked only when it is shared config. `kujo-workflows/agency-verified-fix-loop/scripts/run-loop.sh` directs pre-fix Lens captures into `.casefile-agency-loop/` inside disposable fixture repositories. |
| Intake | `.intake/` including `config/`, `secrets/`, `raw/`, `items/`, `actions/`, `learnings/`, `index/`, `logs/*.jsonl`, `strata/daily/`, `totalrecall/exports/`, `evals/`, `docs/`, `specs/`, and `backups/`; backup files from `intake backup create` are caller-selected and default under the store | `.intake/.env` and `.intake/secrets/` are intentionally local; copy shared source definitions into reviewed config/docs rather than tracking the live store. |
| Redact | `.redact/runs/<timestamp>/` with `run.json`, manifests, detections/decisions/transformations/warnings JSONL, verifier reports, policy snapshots, hashes; default sanitize outputs beside inputs as `*.redacted`, `*.redacted.md`, or `*.redacted.txt`; local transcript-audit wrappers may write `redact-audit/`, `redact-transcript-audit/`, and `transcript.report.json` | `--audit-dir` and `--out` are caller-selected; sample redacted fixtures may be intentionally tracked when reviewed. Transcript policies such as `transcript.policy.yaml` are caller-authored and should be tracked only when they are intentional shared policy. |
| PatchBrief | stdout by default; common redirected files are `patchbrief.md` and `PATCHBRIEF.md`; dogfood artifacts under `.dogfood/`; generated MCP artifacts under `.kujo-mcp/` | Current CLI does not write a default file unless output is redirected by caller/wrapper. |
| ChangeBucket | caller-selected `--output`, commonly `CHANGE_BUCKET.md` or `.cinch/artifacts/changebucket.md` | No default file output without `--output`. |
| Scent | `.scent/packs/<stamp>/`, `out/`, `context.md`, `context.json`, `files.json`, `manifest.json`, `metadata.json`, `redactions.json` | Default pack path is `.scent/packs/<timestamp>` when `--out` is omitted. |
| Capsule | `capsule-output/` by default, or caller-selected `--out`; writes `capsule.json`, `capsule.md`, and `manifest.json` depending on `--format`; test examples commonly use `tmp/<case>/` | Review and track golden capsules only when they are deliberate fixtures. |
| Scout | `results/<project>-<timestamp>/`, `FILE_TREE.md`, `README.md`, `llms.txt`, `AGENTS.md`, `CHECKLIST.md`, `intelligence.json`, `security.sarif`, `security.jsonl`, `index.json`, `packages/*.json`, `scan_manifest.json`, `scout-baseline.json` | Baselines may be intentionally committed in some repos. |
| SearchBridge | Caller-selected evidence exports via `--out`, temporary `--out.partial-<timestamp>` files during JSONL streaming, caller-selected replay/cache directories via `--cache-dir`, release SBOM/provenance artifacts under `dist/`, and Rust generated-SDK Cargo build output under `sdk/rust/target/` | Generated SDK source under `sdk/`, schemas, golden fixtures, and reviewed release files can be source-worthy; keep only local build/cache/export artifacts ignored by default. |
| MCP generator | `.mcp/generated-server/`, `.mcp/artifacts/`, `repo-profile.json`, generated server scaffold, review artifacts, `.kujo_cache/`, `mcp-calls.log`, `server.log`, `test_output.log` | Defaults live inside the target repository. |
| Spec | Caller-selected outputs from `spec init`, `render`, `export`, and `convert`, commonly `specs/*.spec.yml`, `docs/specs/*.md`, `artifacts/*.json`, `eval_suite.json`, or `work-unit.json`; release script emits `dist/kujo-spec-vX.Y.Z.*` | Specs, generated command inventories, and reviewed release artifacts are often source-worthy; do not add a blanket `/artifacts/` ignore. |
| Fence | `fence-baseline.json`, optional import-extraction cache `.fence/cache-v1.json`, caller-selected reports such as `FENCE_REPORT.md`, `fence.sarif`, `architecture.mmd`, `architecture.dot`; PackWrite-style `agent/` packs in some repos | Baseline/config files may be intentionally tracked during rollout. The shared block ignores only the known cache file under `.fence/`, so repo-specific `.fence/**` report roots still need local policy. |
| Eval | `eval_results/`, local docs/test output roots such as `.eval_quickstart`, `.eval_readme_parity`, `.eval_enterprise_*`, `.eval_smoke_out`, and interrupted scratch files such as `.eval_*_suite.json` or `.eval_*_payload.txt`; `snapshots/`, `artifact-manifest.json`, `cli-summary.json`, `eval-report.md`, `eval-report.md.cache.json`, `history.json`, `last_failures.json`, `last_run.json`, `summary.json`, `badge.json`, `benchmarks.json`, `benchmark_trend.csv` | `.gitkeep` files may be tracked intentionally. If a repository intentionally shares a hidden `.eval_*` config, add a repo-specific negation after the block. |
| Lens | `.lens/runs/<timestamp>/`, `.lens/baselines/`, `lens-report.json`, `proof/` | `.lens.toml` is config; track only when shared. |
| Muzzle | `.muzzle/logs/`, `.muzzle/reports/`, `.muzzle/manifests/`, `.muzzle/workflows/`, `.dogfood/` | `manifests/` and `workflows/` can be tracked examples; ignore logs/reports by default. |
| Dispatch | `outputs/`, `outputs/.dispatch-run-index.json`, `outputs/run-*/state.json`, `trace.json`, `trace.md`, `report.json`, `report.md`, exported bundles | `--output-root` can redirect all run artifacts. |
| Kujo workflow runners | `.runs/<timestamp>/`, `.work/<timestamp>/`, `.kujo/runs/<run-id>/`, `.kujo/feature-cards/<card-id>/`, `.kujo/agency/auth/<site>/<role>.storage-state.json`, `.health.json`, `.login.log` | Workflow kits use these roots for local proof packets, disposable workspaces, resumable run state, handoffs, browser/login evidence, and logs. `.kujo/agency/sites/*.yml` profiles are caller-authored config and should be tracked or ignored per repository policy rather than covered by the shared block. |
| PackWrite | `agent/`, `DEEPSEEK_START.md`, `CODEX_REVIEW_PROMPT.md`, `MASTER.md`, `HANDOFF.md`, `REVIEW_CHECKLIST.md`, `TODO.md`, `DECISIONS.md`, `phases/*.md` | Default output directory is `agent`; config can change it. |
| Kennel | `.kennel_tmp/`, `kennel_packages/`, `.kennel_installer_trash/`, `.kennel_tokens.json`, `hosted-registry/`, local `tokens.json`, benchmark/review logs | `kennel.lock` is normally source-worthy. |
| ShipCheck | `shipcheck-report.json`, `shipcheck-report.md`, `eval_results/`, `.dogfood/shipcheck/` | CLI primarily writes to stdout unless redirected. |
| Concord | `.loop-engineering/`, `.dogfood/` | Reports are stdout unless redirected by caller. |
| Relay | `.relay/`, `.relay/runs/<mission>/`, `agent/`, `ledger/`, `workspace/`, `packet-manifest.json`, `tool-results.json`, `state.json`, `report.json`, `relay-contract-*/` contract-test roots when `RELAY_TEST_TMP_ROOT` points at the repo | Existing local repo has user changes outside this inventory. |
| WorkCell | `.workcell/runs/`, `.runledger/`, `.casefile/`, `.loop-engineering/`, generated `src/artifacts/` and `src/output/` during some flows | Keep source `src/artifacts` only if intentionally authored. |
| Tribunal | `tribunal-runs/`, `.strata-tmp/`, per-run `artifact-manifest.json`, `checkpoint.json`, `context.md`, `docket.md`, `decision-packet.md`, `events.jsonl`, `receipt.json`, `record.json`, `ruling.md`, `prompts/`, `testimony/` | Run archives are local evidence unless deliberately published. |
| AssetWorks | `.assetworks/` with `metadata.json`, immutable `records/*.json`, append-only `history/*.json`, and per-record `locks/*.lock`; caller-selected export files such as `assetworks-export.json` | Default state is `.assetworks/`; `--state` and `--output` are caller-selected, so reviewed exported records should be tracked only when intentionally published. |
| BluePencil | `.bluepencil/` with `metadata.json`, immutable `records/*.json`, append-only `history/*.json`, and per-record `locks/*.lock`; caller-selected export files | Default state is `.bluepencil/`; calibration fixtures under `fixtures/calibration/` are source-worthy and should remain tracked. |
| Dossier | `.dossier/` with `metadata.json`, immutable `records/*.json`, append-only `history/*.json`, and per-record `locks/*.lock`; caller-selected packet/export files | Default state is `.dossier/`; evidence fixtures and citation examples are source-worthy when reviewed. |
| GalleyPack | `.galleypack/` with `metadata.json`, immutable `records/*.json`, append-only `history/*.json`, and per-record `locks/*.lock`; caller-selected export/build artifacts | Default state is `.galleypack/`; source manuscripts and reviewed package fixtures are not blanket-ignored. |
| PressWire | `.presswire/` with local approval/publication records and history; caller-selected exports such as `presswire-export.json` | Default state is `.presswire/`; `--act --yes` effect commands still require exact approval, and release/publication artifacts should be tracked only when intentionally published. |
| ReaderSignal | `.readersignal/` with `metadata.json`, immutable `records/*.json`, append-only `history/*.json`, and per-record `locks/*.lock`; caller-selected exports such as `readersignal-export.json` | Default state is `.readersignal/`; measurement inputs and reviewed fixtures remain repo policy decisions. |
| StoryDesk | `.storydesk/`, optional `.storydesk-sqlite/`, JSON state `records/`, `history/`, `locks/`, optional `storydesk.sqlite`, caller-selected packet checkpoints such as `packet.checkpoint.json`, and caller-selected packet/export files | Default JSON state is `.storydesk/`; the SQLite adapter is opt-in. Packet checkpoints and exports are caller-selected, so track only deliberate handoffs. |
| VersionSeal | `.versionseal/` with `metadata.json`, immutable approval records, append-only history, and per-record locks; caller-selected exports such as `versionseal-export.json` | Default state is `.versionseal/`; public key fixtures and release-policy examples are source-worthy. |
| ContentGraph | `.contentgraph/<run>/`, `.contentgraph/deterministic/`, `graph.json`, `nodes.jsonl`, `edges.jsonl`, `clusters.json`, `overlaps.json`, `orphan-candidates.json`, `link-opportunities.json`, `analysis.json`, `metadata.json`, `report.md`, `vector-cache.jsonl`, `adapter-cache.jsonl`, `manifest.json`, optional `telemetry.json`, and caller-selected exports such as GraphML/SARIF | `--out` is caller-selected; default non-deterministic runs go under `.contentgraph/<timestamp>` and deterministic runs under `.contentgraph/deterministic`. Golden runs under `fixtures/golden/` are intentional source fixtures. |
| Source | `.source/` JSON/SQLite local data store, content-addressed `artifacts/<prefix>/<hash>`, audit log/store files, and local backup staging roots such as `.source-backup-*` under a caller-selected backup output; observed local `audit-results/` | `SOURCE_DATA` can redirect the store. Backup/export directories are caller-selected; reviewed Source docs and fixtures remain tracked. |
| Ward | `data/ward.db`, JSON fallback state such as `data/alerts.json`, dated `data/artifacts/<YYYY-MM-DD>/` reports including `report.md` and `alerts.json`, generated `dashboard/index.html`, and `logs/` | Config examples under `config/*.example.yaml` are source-worthy; live `config/repos.yaml`, token-bearing env, generated dashboard, local alert DB, and report artifacts are local operator state. |
| RAG | `data/rag_index*.json`, `results/`, `results/**/*.json`, `results/**/*.log`, `results/**/*.db*`, `results/privacy/`, `release-manifest.json` | Some release/docs artifacts may be intentional; default local index is generated. |
| Watchdog | `data/watchdog.db`, SQLite sidecars such as `data/watchdog.db-shm`, `data/watchdog.db-wal`, and `data/watchdog.db-journal`, `data/backups/watchdog-backup-*.db*`, `backups/*.db*`, `.watchdog-backup-result-*.json`, `watchdog_proxy_config.json`, `tmp/*.db*`, `tmp/*.log`, `tmp/*.json`, copied browser/vendor assets in `tmp/` | Dashboard source remains tracked elsewhere; local proxy config can contain operational endpoint/auth settings. |
| AI Chat | `.env`, `data/ai_chat.db`, `data/audit.log`, weekly tool-audit reports under `data/audits/`, `data/backups/*.db*`, `data/benchmark-runs/`, `data/tool-artifacts/`, `node_modules/` | Secrets stay untracked; browser screenshots, weekly audit reports, and benchmark telemetry are local evidence unless intentionally published. |
| CMS | `results/*.db*`, `results/*.log` | Test/integration state. |
| CRUD API | `.tmp/*.db*`, `.tmp/*.log`, `.tmp/*.body`, `.tmp/*.out`, frontend build output | Local smoke-test state. |
| SSG and Kujo Docs | `output/`, `.output-*`, `logs/`, `tmp/`, `.kujo-post-manifest.txt` | Generated static site and build logs. Kujo Docs writes its canonical site to `output/`; interrupted or partial local builds have been observed under `.output-partial-*` and `.output-incomplete-*`, matching the repo-local `.output-*` ignore. |
| Howl | `dist/howl/*.html`, `dist/howl/*.md`, `dist/howl/*.svg`, `.howl-social/*.svg`, `tmp_test_*/` from the shell/unit harness, and transient `--interpreter/tmp_test_*/` scratch dirs | Rendered cards/gallery, social-card exports, and local test harness output. |
| SiteProbe | Caller-selected crawl roots, commonly `.siteprobe/<run>/`, with `run.json`, `pages.jsonl`, `links.jsonl`, `robots.json`, `sitemaps.jsonl`, `manifest.json`, and `report.md`; doc generation scratch root `.siteprobe-docgen-tmp/` | `--out` is caller-selected. Generated API docs copied into `docs/generated/` can be source-worthy when reviewed; keep those tracked by repo policy. |
| SiteKit | `dist/` bundle (`sitekit.css`, `sitekit.js`, `fonts/`, distribution README), `css/generated/*.css`, `tests/visual/component-snapshot.json`, `artifacts/release/sitekit-v*.tar.gz`, `artifacts/release/*.sha256`, and `artifacts/browser/` Playwright reports | SiteKit intentionally tracks `dist/`, `css/generated/`, and the component snapshot as source-vendored v1 artifacts in its own repo. Release archives and browser reports are local verification output and should stay ignored. Consumers may intentionally vendor `dist/`; keep that as a repo-specific decision. |
| Zelus | `.zelus/engagement-manifest.json`, `.zelus/campaign.json`, `.zelus/reference-run/`, plus caller-selected `--out` manifests, hypotheses, and reference/eval run directories | Current examples usually write to `/tmp`; ignore `.zelus/` when run inside a repo. |
| Kujo Commerce | `.kujo-commerce/` prepared content/assets workspace, generated site output such as `output/`, `_kujo/commerce/catalog.json` inside the output tree, `.kujo-bin/` downloaded Kujo binaries/checksums from clean-clone scripts, and `.wrangler/` Cloudflare local state | `kujo-commerce.yml`, `kujo-ssg.yml`, and `wrangler.toml` are source-worthy config. The shared block covers local generated/deployment state while leaving reviewed vendored assets such as `assets/sitekit/` to repo policy. |
| WebOps dashboard/toolchain | `.webops/` including dashboard SQLite files, site profiles, runs, findings, history, baselines, reports, and actions | Some profile definitions may become shared config; add repo-specific negations when a profile is intentionally source-controlled. |
| Codebase Cleanup workflow | `.cleanup-runs/<timestamp>/` or named output roots containing `REPORT.md`, `report.json`, `cleanup-plan.json`, and integration logs | `--output` can redirect the run root. Cleanup reports are local review evidence unless deliberately published. |
| AI SDK muzzle benchmark workflow | `.suites/<timestamp>/` with `summary.json`, `review.html`, `run-dirs.txt`, and per-trial logs beside the normal `.runs/<timestamp>-trial-XX/` folders | Suite output is local benchmark evidence. |
| TruthLens | `.model-build/` converted model files, examples, and local virtualenvs; `.model-candidates/` downloaded candidate models; `.benchmark-cache/` image cache; `.eval-results/` Eval output; `dist/` unpacked browser extension; `truthlens-extension.zip` release archive | `eval.json` and extension source are source-worthy. The generated extension package and local model/cache/eval roots should stay untracked unless a release artifact is deliberately published. |
| Leash | `audit.log` and `daemon/audit.log` from local daemon audit sinks; daemon build output under `daemon/target/` | Shared daemon config examples are source-worthy. Audit logs are local operational evidence. |
| Agents SDK | file-backed artifact/trace stores write under caller-provided roots, often `/tmp`; no repo-local default artifact root found | Ignore chosen local roots where configured. |
| AI SDK | `artifacts/security/integrity-manifest.sha256` and other local release/security artifacts | CI may upload these; local source control should ignore unless intentionally reviewed. |
| TotalRecall | Caller-selected reports such as `artifacts/run-report.json`, coverage checks under `artifacts/coverage/` and `.coverage`, and optional Markdown/HTML/local-index destination directories configured via `TOTALRECALL_*_OUTPUT_DIR` | Destination roots are caller-selected and can be source-worthy exports; only the known local coverage artifact roots are in the shared block. |
| Benchmarks System | `results/<run-id>/`, `results/<run-id>/source_evidence/`, generated static dashboard `dist/`, and zipped dashboard bundles under `dist-build/` | `BENCHMARK_EXECUTION_KIT/` and `BENCHMARK_REVIEW_KIT/` are source-worthy operator prompts. The benchmark runner output directory is caller-selected; the local dashboard exporter defaults to `results/` and writes `dist/`. |

## Reusable Ignore Block

Use `config/kujo-tool-artifacts.gitignore` as the canonical starting block. Repositories should keep source-worthy config files tracked by adding negated rules after the block, for example:

```gitignore
!fence.toml
!fence-baseline.json
!.lens.toml
!casefile.toml
```

The guard evaluates the repository's effective ignore rules, so reviewed source
fixtures may also use a repo-specific negation. Kujo keeps `tests/*.out` as
compiler-output fixtures with `!/tests/*.out`. Deleting an ignored artifact is
allowed; adding or modifying one remains blocked.

The block also carries a few exact or broad support patterns that intentionally
serve multiple rows in the inventory:

- `/runledger-report.md` mirrors `RUNLEDGER_REPORT.md` for lower-case redirected RunLedger reports.
- `/.cinch/pack/` covers generated Cinch wrapper packs beside `/.cinch/artifacts/`.
- `/.runs/` covers timestamped Kujo workflow-runner packets; `/.work/` covers disposable workflow workspaces such as `agency-verified-fix-loop/.work/<timestamp>/`.
- `/.kujo/runs/` covers Agency Runner proof packets; `/.kujo/feature-cards/` covers Feature Card workflow task, spec, context, proof, brief, ledger, handoff, and log packets; `/.kujo/agency/auth/` covers saved browser-session state and login evidence while leaving caller-authored `.kujo/agency/sites/*.yml` profiles to repo policy.
- `/sdk/rust/target/` covers SearchBridge generated-SDK Cargo build output while leaving generated SDK source under `sdk/` available for repository-specific tracking.
- `/.fence/cache-v1.json` covers Fence's optional import-extraction cache without blanket-ignoring caller-selected `.fence/**` report roots.
- `/data/*.db`, `/data/*.db-shm`, `/data/*.db-wal`, `/data/*.db-journal`, `/data/*.sqlite-journal`, `/data/*.sqlite3-journal`, `/data/*.log`, `/*.db-shm`, `/*.db-wal`, `/*.db-journal`, `/*.sqlite`, `/*.sqlite-journal`, `/*.sqlite3`, and `/*.sqlite3-journal` cover SQLite/log sidecars emitted by Watchdog, AI Chat, CMS, CRUD API, RAG, and related local state.
- `/data/audits/` covers AI Chat weekly tool-audit JSON/Markdown reports written by `scripts/weekly-tool-audit.js`.
- `/tests/tmp/`, `/tests/*.out`, `*.tmp-*`, and `*.bak` cover local test harness output and atomic-write leftovers.
- `/.eval_*` covers Eval quickstart, parity, smoke, benchmark, and interrupted test scratch outputs named that way in docs and tests.
- `/.output-*/` covers interrupted or partial Kujo Docs/SSG static-site builds beside the canonical `/output/` root.
- `/dist-build/` covers generated dashboard/package archives beside `/dist/` static output.
- `/artifacts/release/` and `/artifacts/browser/` cover SiteKit release archives/checksums and Playwright reports while avoiding a blanket `/artifacts/` rule.
- `/tmp_test_*/` and `/--interpreter/tmp_test_*/` cover Howl harness scratch output created at the repository root and under the Kujo interpreter-flag working directory seen in local ignored files.
- `/.howl-social/` covers generated Howl social-card SVG exports.
- `/.siteprobe/` covers caller-selected SiteProbe crawl roots used by docs and examples; `/.siteprobe-docgen-tmp/` covers the temporary Kujo docgen root used before copying reviewed API docs into `docs/generated/`.
- `/.kujo-commerce/` covers Kujo Commerce's prepared content/assets workspace; `/.kujo-bin/` covers downloaded Kujo release binaries used by clean-clone scripts; `/.wrangler/` covers Cloudflare Pages local deploy state.
- `/.webops/`, `/.cleanup-runs/`, and `/.suites/` cover WebOps dashboard state, Codebase Cleanup reports/plans, and aggregate benchmark-suite evidence beside existing workflow runner roots.
- `/.benchmark-cache/`, `/.eval-results/`, `/.model-build/`, `/.model-candidates/`, and `/truthlens-extension.zip` cover TruthLens local model conversion, benchmark cache, Eval output, and extension packaging artifacts.
- `/artifacts/coverage/` covers TotalRecall's local coverage-report artifact without adding a blanket `/artifacts/` ignore.
- `*.log` covers Leash local audit sinks such as `audit.log` and `daemon/audit.log` while leaving config examples tracked.
- `/redact-audit/`, `/redact-transcript-audit/`, and `/transcript.report.json` cover local Redact transcript audit runs while leaving caller-authored transcript policies to repo-specific decisions.
- `/.assetworks/`, `/.bluepencil/`, `/.dossier/`, `/.galleypack/`, `/.presswire/`, `/.readersignal/`, `/.storydesk/`, `/.storydesk-sqlite/`, and `/.versionseal/` cover default local state roots for the newer local-first Kujo record tools while leaving caller-selected exports and reviewed fixtures to repo policy.
- `/.contentgraph/` covers default ContentGraph run/cache output; committed `fixtures/golden/run/` output remains a source fixture.
- `/.source/` and `/audit-results/` cover Source's local data/evidence roots while leaving explicit backup/export destinations to caller policy.
- `/data/artifacts/`, `/data/alerts.json`, `/data/ward.db`, and `/dashboard/` cover Ward's local alert state, dated reports, SQLite store, and generated dashboard without ignoring its source-worthy `config/*.example.yaml` files.
- `/.coverage` covers TotalRecall's local Python harness coverage marker beside the existing `/artifacts/coverage/` report root.

## Verification

Checked source and local generated artifacts with:

```sh
find /Users/robertdevore/2026/Kujolang/kujo-repos -maxdepth 3 -type d -name .git -print
rg -n "Usage:|--out|--output|--output-dir|--artifacts|--ledger|--snapshot|--baseline|write_file\\(|create_dir\\(|ensure_dir" \
  -g '*.kujo' -g '*.md' -g '*.rs' -g '*.ts' \
  --glob '!**/target/**' --glob '!**/node_modules/**'
git ls-files -o -i --exclude-standard
```
