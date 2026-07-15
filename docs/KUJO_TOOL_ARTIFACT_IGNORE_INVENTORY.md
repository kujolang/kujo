# Kujo Tool Artifact Ignore Inventory

Status: research snapshot
Last updated: 2026-07-15

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
| CaseFile | `.casefile/<case-id>/case.{md,json}`, `command.txt`, `environment.json`, `git-*.txt`, `stdout.log`, `stderr.log`, `combined.log`, `reproduction.md`, `handoff.md` | `casefile.toml` is created by `init`; keep it tracked only when it is shared config. |
| PatchBrief | stdout by default; common redirected files are `patchbrief.md` and `PATCHBRIEF.md`; dogfood artifacts under `.dogfood/`; generated MCP artifacts under `.kujo-mcp/` | Current CLI does not write a default file unless output is redirected by caller/wrapper. |
| ChangeBucket | caller-selected `--output`, commonly `CHANGE_BUCKET.md` or `.cinch/artifacts/changebucket.md` | No default file output without `--output`. |
| Scent | `.scent/packs/<stamp>/`, `out/`, `context.md`, `context.json`, `files.json`, `manifest.json`, `metadata.json`, `redactions.json` | Default pack path is `.scent/packs/<timestamp>` when `--out` is omitted. |
| Scout | `results/<project>-<timestamp>/`, `FILE_TREE.md`, `README.md`, `llms.txt`, `AGENTS.md`, `CHECKLIST.md`, `intelligence.json`, `security.sarif`, `security.jsonl`, `index.json`, `packages/*.json`, `scan_manifest.json`, `scout-baseline.json` | Baselines may be intentionally committed in some repos. |
| MCP generator | `.mcp/generated-server/`, `.mcp/artifacts/`, `repo-profile.json`, generated server scaffold, review artifacts, `.kujo_cache/`, `mcp-calls.log`, `server.log`, `test_output.log` | Defaults live inside the target repository. |
| Fence | `fence-baseline.json`, caller-selected reports such as `FENCE_REPORT.md`, `fence.sarif`, `architecture.mmd`, `architecture.dot`; PackWrite-style `agent/` packs in some repos | Baseline/config files may be intentionally tracked during rollout. |
| Eval | `eval_results/`, `snapshots/`, `artifact-manifest.json`, `cli-summary.json`, `eval-report.md`, `eval-report.md.cache.json`, `history.json`, `last_failures.json`, `last_run.json`, `summary.json`, `badge.json`, `benchmarks.json`, `benchmark_trend.csv` | `.gitkeep` files may be tracked intentionally. |
| Lens | `.lens/runs/<timestamp>/`, `.lens/baselines/`, `lens-report.json`, `proof/` | `.lens.toml` is config; track only when shared. |
| Muzzle | `.muzzle/logs/`, `.muzzle/reports/`, `.muzzle/manifests/`, `.muzzle/workflows/`, `.dogfood/` | `manifests/` and `workflows/` can be tracked examples; ignore logs/reports by default. |
| Dispatch | `outputs/`, `outputs/.dispatch-run-index.json`, `outputs/run-*/state.json`, `trace.json`, `trace.md`, `report.json`, `report.md`, exported bundles | `--output-root` can redirect all run artifacts. |
| PackWrite | `agent/`, `DEEPSEEK_START.md`, `CODEX_REVIEW_PROMPT.md`, `MASTER.md`, `HANDOFF.md`, `REVIEW_CHECKLIST.md`, `TODO.md`, `DECISIONS.md`, `phases/*.md` | Default output directory is `agent`; config can change it. |
| Kennel | `.kennel_tmp/`, `kennel_packages/`, `.kennel_installer_trash/`, `.kennel_tokens.json`, `hosted-registry/`, local `tokens.json`, benchmark/review logs | `kennel.lock` is normally source-worthy. |
| ShipCheck | `shipcheck-report.json`, `shipcheck-report.md`, `eval_results/`, `.dogfood/shipcheck/` | CLI primarily writes to stdout unless redirected. |
| Concord | `.loop-engineering/`, `.dogfood/` | Reports are stdout unless redirected by caller. |
| Relay | `.relay/`, `.relay/runs/<mission>/`, `agent/`, `ledger/`, `workspace/`, `packet-manifest.json`, `tool-results.json`, `state.json`, `report.json` | Existing local repo has user changes outside this inventory. |
| WorkCell | `.workcell/runs/`, `.runledger/`, `.casefile/`, `.loop-engineering/`, generated `src/artifacts/` and `src/output/` during some flows | Keep source `src/artifacts` only if intentionally authored. |
| Tribunal | `tribunal-runs/`, `.strata-tmp/`, per-run `artifact-manifest.json`, `checkpoint.json`, `context.md`, `docket.md`, `decision-packet.md`, `events.jsonl`, `receipt.json`, `record.json`, `ruling.md`, `prompts/`, `testimony/` | Run archives are local evidence unless deliberately published. |
| RAG | `data/rag_index*.json`, `results/`, `results/**/*.json`, `results/**/*.log`, `results/**/*.db*`, `results/privacy/`, `release-manifest.json` | Some release/docs artifacts may be intentional; default local index is generated. |
| Watchdog | `data/watchdog.db`, `tmp/*.db*`, `tmp/*.log`, `tmp/*.json`, copied browser/vendor assets in `tmp/` | Dashboard source remains tracked elsewhere. |
| AI Chat | `.env`, `data/ai_chat.db`, `data/audit.log`, `data/backups/*.db*`, `node_modules/` | Secrets stay untracked. |
| CMS | `results/*.db*`, `results/*.log` | Test/integration state. |
| CRUD API | `.tmp/*.db*`, `.tmp/*.log`, `.tmp/*.body`, `.tmp/*.out`, frontend build output | Local smoke-test state. |
| SSG | `output/`, `logs/`, `tmp/`, `.kujo-post-manifest.txt` | Generated static site and build logs. |
| Howl | `dist/howl/*.html`, `dist/howl/*.md`, `dist/howl/*.svg`, transient `--interpreter/tmp_*` | Rendered cards/gallery. |
| Agents SDK | file-backed artifact/trace stores write under caller-provided roots, often `/tmp`; no repo-local default artifact root found | Ignore chosen local roots where configured. |
| AI SDK | `artifacts/security/integrity-manifest.sha256` and other local release/security artifacts | CI may upload these; local source control should ignore unless intentionally reviewed. |

## Reusable Ignore Block

Use `config/kujo-tool-artifacts.gitignore` as the canonical starting block. Repositories should keep source-worthy config files tracked by adding negated rules after the block, for example:

```gitignore
!fence.toml
!fence-baseline.json
!.lens.toml
!casefile.toml
```

## Verification

Checked source and local generated artifacts with:

```sh
find /Users/robertdevore/2026/Kujolang/kujo-repos -maxdepth 3 -type d -name .git -print
rg -n "Usage:|--out|--output|--output-dir|--artifacts|--ledger|--snapshot|--baseline|write_file\\(|create_dir\\(|ensure_dir" \
  -g '*.kujo' -g '*.md' -g '*.rs' -g '*.ts' \
  --glob '!**/target/**' --glob '!**/node_modules/**'
git ls-files -o -i --exclude-standard
```
