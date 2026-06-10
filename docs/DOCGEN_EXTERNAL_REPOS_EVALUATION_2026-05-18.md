# DocGen External Repos Evaluation (2026-05-18)

## Scope

Validated `kujo docgen` against these repositories:

- `/Users/robertdevore/Documents/Kujolang/kujo-repos/ai-sdk`
- `/Users/robertdevore/Documents/Kujolang/kujo-repos/mcp`
- `/Users/robertdevore/Documents/Kujolang/kujo-repos/scout`

Goals:

- Confirm generation works end-to-end.
- Confirm docgen tests/contracts still pass.
- Identify practical limitations and best next fixes.

## Commands Run

Generation (non-strict):

```bash
target/debug/kujo docgen <repo> \
  --out-dir docs/generated/external/<name> \
  --format all \
  --emit-ai-tasks \
  --search-index \
  --source-links \
  --include-private \
  --json
```

Strict gate run:

```bash
target/debug/kujo docgen <repo> \
  --out-dir docs/generated/external/<name>-strict \
  --format all \
  --emit-ai-tasks \
  --search-index \
  --source-links \
  --include-private \
  --fail-on-undocumented \
  --fail-on-broken-links \
  --fail-on-warnings \
  --json
```

Docgen-related tests:

```bash
cargo test --test docgen_universal -- --nocapture
cargo test --test cli_json_contracts docgen_json_contract_is_stable -- --nocapture
cargo test --test package_module_workflow_integration package_module_workflow_end_to_end_contract -- --nocapture
```

## Results

### Generation status

- `ai-sdk`: success, `item_count=329`, `diagnostics=0`, `warnings=0`
- `mcp`: success, `item_count=328`, `diagnostics=0`, `warnings=0`
- `scout`: success, `item_count=311`, `diagnostics=0`, `warnings=0`

### Artifacts present for all 3 repos

- `index.html`
- `docgen.md`
- `docgen.json`
- `docgen-gaps.json`
- `docgen-capabilities.json`
- `docgen-ai-tasks.md`
- `search-index.json`
- `symbol-index.json`

### Gap scan highlights (non-strict outputs)

- `ai-sdk`: `MissingDocs=25`, `MissingExamples=25`
- `mcp`: `MissingDocs=14`, `MissingExamples=14`
- `scout`: `MissingDocs=22`, `MissingExamples=22`

### Strict gate results

All three strict runs failed on undocumented symbols only:

- `ai-sdk`: `19 undocumented public symbols detected`
- `mcp`: `16 undocumented public symbols detected`
- `scout`: `1 undocumented public symbols detected`

No strict run reported broken links or warnings.

### Workstream A Task 2 follow-up (2026-05-18)

After updating Kujo top-level function visibility to require explicit `pub`, strict-mode metrics changed as follows:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 19 | 0 | -19 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 16 | 0 | -16 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 1 | 0 | -1 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: strict failures previously attributed to undocumented public symbols were eliminated for these repos by removing top-level visibility false positives.

### Workstream A Task 3 follow-up (2026-05-18)

After updating Kujo container-member visibility to keep symbols under private containers private (for example `pub` methods on private structs and variants on private enums), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: the Workstream A task 3 visibility hardening did not change strict gate counts on these three repositories (their current symbol sets do not rely on private-container member edge cases), while preserving zero broken links and zero warnings.

### Workstream A Task 4 follow-up (2026-05-18)

After adding explicit `public_only` visibility-matrix regression coverage in `tests/docgen_universal.rs`, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 4 is test-coverage hardening; external strict gate counts remain stable while regression protection for visibility classification increased.

### Workstream A Task 1 follow-up (2026-05-18)

After documenting the Kujo DocGen visibility policy in `docs/DOCGEN.md` (top-level `pub` requirement plus container-aware member visibility rules), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 1 is policy-definition/documentation hardening and does not change external strict gate counts.

### Workstream B Task 1 follow-up (2026-05-18)

After expanding Kujo extraction coverage to include `async func` / `pub async func` declarations in the Kujo adapter, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: the new async-declaration extraction support did not change strict external counts for these repos, but it closes a real Kujo syntax extraction gap and is covered by new async visibility and strict-gate regressions.

### Workstream B Task 2 follow-up (2026-05-18)

After adding fixture-driven Kujo extraction edge-case regressions (async visibility + strict-gate fixtures under `tests/fixtures/docgen/`), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 2 is test-hardening; external strict gate metrics remain stable while extraction edge coverage is now fixture-locked.

### Workstream B Task 3 follow-up (2026-05-18)

After evaluating parser-backed extraction versus the current regex-first hybrid path and documenting the decision, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 3 is architecture-decision documentation only; strict external gate counts remain stable while the hybrid extraction strategy and parser-assisted fallback criteria are now explicit.

### Workstream C Task 1 follow-up (2026-05-18)

After adding Kujo inline doc extraction support for additional comment styles (`//!` and `/** ... */`) while keeping plain `/* ... */` excluded, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 1 improves Kujo documentation attachment coverage for supported comment styles without changing strict gate counts on these repositories.

### Workstream C Task 2 follow-up (2026-05-18)

After strengthening Kujo comment-to-symbol attachment heuristics to skip decorator/attribute lines between docs and symbols, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 2 improves attachment reliability in decorator-adjacent Kujo code paths while preserving strict gate stability on the external validation set.

### Workstream C Task 3 follow-up (2026-05-18)

After adding spacing/proximity regression tests for Kujo doc attachment edge cases (blank-line spacing, regular-comment breaks, and nearest-block precedence), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 3 is coverage hardening; strict external gate metrics remain stable while spacing/proximity behavior is now explicitly regression-locked.

### Workstream D Task 1 follow-up (2026-05-18)

After adding explicit discovery skip diagnostics for max file size, max depth, and max file count limits, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 1 improves discovery transparency without changing strict gate counts on the external validation set.

### Workstream D Task 2 follow-up (2026-05-18)

After adding discovery skip summary counters by reason to the docgen CLI JSON contract (`discovery_skip_counts` with `max_file_size`, `max_depth`, `max_files`), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 2 is summary-contract transparency hardening; strict external gate counts remain stable while per-reason discovery skip counters are now available for CI/reporting.

### Workstream D Task 3 follow-up (2026-05-18)

After enforcing deterministic diagnostic sorting for discovery and project diagnostics, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 3 is deterministic-ordering hardening; strict external gate counts remain stable while diagnostics ordering is now explicitly regression-locked for repeatable CI diffs.

### Workstream E Task 1 follow-up (2026-05-18)

After locking default link validation to local-file existence checks (with local anchor/query suffixes ignored and external/mailto links skipped by default), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 1 preserves strict gate stability while making default link-check behavior explicit and regression-locked.

### Workstream E Task 2 follow-up (2026-05-18)

After adding optional local-anchor validation mode (`--validate-local-anchors`) while preserving default link-validation behavior, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 2 introduces an opt-in stricter local-anchor check mode without affecting default strict-gate results on the external validation set.

### Workstream E Task 3 follow-up (2026-05-18)

After adding optional external-link validation mode with allowlist and timeout controls (`--validate-external-links`, `--external-link-allowlist`, `--external-link-timeout-ms`), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 3 adds opt-in external validation controls while preserving default strict-gate stability on the external validation set.

### Workstream E Task 4 follow-up (2026-05-18)

After adding explicit link-validation mode diagnostics and gate-failure breakdowns (including warnings for external-mode misconfiguration and per-mode broken-link counts), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 4 improves mode-behavior clarity through explicit diagnostics and stable contract coverage, while preserving strict external gate stability on the validation set.

### Workstream F Task 1 follow-up (2026-05-18)

After adding separate summary counts for project symbols and builtin symbols (`project_symbol_count`, `builtin_symbol_count`) in DocGen run summary and CLI JSON output, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 1 improves summary signal quality for CI/dashboards without changing strict gate outcomes on the external validation set.

### Workstream F Task 2 follow-up (2026-05-18)

After adding deterministic per-kind symbol counts in DocGen summary output (`symbol_kind_counts`, including function/method/struct/enum/builtin categories), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 2 improves symbol-shape observability for quality dashboards while preserving strict gate stability on the external validation set.

### Workstream F Task 3 follow-up (2026-05-18)

After adding a stable machine-readable dashboard summary block in `kujo docgen --json` (`summary` with `schema_version=docgen-summary/v1` plus synchronized core counters), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: task 3 establishes a versioned dashboard summary contract for automation consumers without changing strict gate outcomes on the external validation set.

### QA Hardening Task DG-QA-001 follow-up (2026-05-18)

After hardening external-link redirect handling to re-validate the host allowlist on every redirect hop (with deterministic `external-redirect-allowlist` diagnostics when redirects leave the allowlist), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: `DG-QA-001` is a security/reliability hardening change in optional external-link mode; strict external validation metrics on this repository set remain stable.

### QA Hardening Task DG-QA-002 follow-up (2026-05-18)

After adding SSRF guardrails for optional external-link validation (default blocking for private/loopback/link-local/multicast direct-IP and DNS-resolved targets, with explicit `--allow-private-network-links` opt-in), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: `DG-QA-002` hardens external-link validation security posture while preserving strict external gate counts on this validation set.

### QA Hardening Task DG-QA-003 follow-up (2026-05-19)

After adding deterministic link-validation resource budgets (`--max-link-checks`, `--max-external-link-checks`, `--max-total-validation-time-ms`) plus JSON-visible truncation counters (`link_validation_skip_counts`) and budget warning diagnostics, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: `DG-QA-003` improves bounded-run reliability and machine-readable budget visibility without changing strict external gate counts on this validation set.

### QA Hardening Task DG-QA-004 follow-up (2026-05-19)

After making discovery encoding-safe (skipping non-UTF-8 source files with deterministic `DOCGEN_DISCOVERY_INVALID_ENCODING` diagnostics and `discovery_skip_counts.invalid_encoding`), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: `DG-QA-004` hardens ingestion resilience and keeps strict external gate metrics stable on this validation set.

### QA Hardening Task DG-QA-005 follow-up (2026-05-19)

After replacing per-call adapter-registry reconstruction with static/lazy adapter metadata + lookup maps, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Micro-benchmark evidence (construction cost):
- Legacy language lookup path reconstructed adapters linearly and could instantiate all 9 adapters for a late match.
- Static lookup path now resolves via lazy map and instantiates only the returned adapter (`1` instantiation), validated by `docgen::adapters::tests::static_lookup_avoids_legacy_full_registry_construction_cost`.

Interpretation: `DG-QA-005` reduces lookup overhead while preserving deterministic strict-gate outputs.

### QA Hardening Task DG-QA-006 follow-up (2026-05-19)

After moving adapter extraction regexes from per-file compilation to static/lazy `OnceLock` caches across Kujo/PHP/Python/TypeScript/JavaScript/Ruby/Go/Haskell/Zig adapters, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Micro-benchmark evidence (`cargo test docgen::adapters::tests::regex_caching_micro_benchmark_evidence -- --nocapture`):
- `src/lib.rs` test run: `cached=35.650514ms`, `legacy=5.420432455s`
- `src/main.rs` test run: `cached=28.938084ms`, `legacy=4.699169071s`

Interpretation: `DG-QA-006` removes repeated regex compilation overhead in adapter extraction hot paths while preserving strict external gate stability on this validation set.

### QA Hardening Task DG-QA-007 follow-up (2026-05-19)

After adding link-validation caching (single external HTTP client reused per run plus parsed local-anchor cache by file path), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Regression-evidence tests:
- `docgen::gaps::tests::repeated_local_anchor_checks_use_cached_file_index_per_path` verifies repeated anchor checks for one target file perform one file read/parse.
- `docgen::gaps::tests::repeated_external_host_checks_reuse_single_http_client` verifies repeated external checks in one run build one HTTP client.

Interpretation: `DG-QA-007` improves repeated link-check throughput and IO efficiency while preserving strict external gate outputs.

### QA Hardening Task DG-QA-008 follow-up (2026-05-19)

After replacing per-symbol full-source known-call-site scans with a one-pass indexed matcher in gap generation, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Regression-evidence tests:
- `docgen::gaps::tests::gap_call_site_index_matches_legacy_order_and_limit_semantics` verifies indexed call-site output matches legacy ordering/limit behavior.
- `docgen::gaps::tests::large_input_call_site_index_scans_each_source_line_once` verifies one-pass line-scan behavior on large multi-file input.

Interpretation: `DG-QA-008` removes repeated full-source rescans from gap call-site discovery while preserving strict external gate outputs and known-call-site contract behavior.

### QA Hardening Task DG-QA-009 follow-up (2026-05-19)

After extracting shared C-style parsing helpers for TypeScript/JavaScript adapter class-scope tracking, brace-depth updates, and JSDoc block parsing, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Regression-evidence tests:
- `docgen::adapters::tests::c_style_adapter_conformance_preserves_typescript_and_javascript_contracts` verifies shared helper refactoring preserves TypeScript/JavaScript symbol and inline-doc attachment expectations.

Interpretation: `DG-QA-009` reduces duplicated adapter parsing logic while preserving strict external gate outputs and language-specific extraction behavior.

### Test results

- `docgen_universal`: passed
- `docgen_json_contract_is_stable`: passed
- `package_module_workflow_end_to_end_contract`: passed

Conclusion: docsgen implementation and contracts are functioning, but quality gates expose documentation coverage gaps.

## Observed Limitations

## 1) Public symbol visibility for Kujo can over-report undocumented API

In `src/docgen/adapters/kujo.rs`, top-level `func` symbols are treated as public unless they are methods without `pub`.
This can classify many internal helper functions as public and inflate `fail-on-undocumented` results.

Impact seen here:

- Strict runs fail in all 3 repos due to undocumented public symbols.

## 2) Kujo adapter extraction is regex-based, not AST-based

`src/docgen/adapters/kujo.rs` uses line regexes for `func`, `struct`, `enum`, constants, and variants.
Complex syntax edge cases can be missed or misclassified compared to parser-backed extraction.

## 3) Inline docs for Kujo currently rely on `///` proximity

Kujo inline docs are attached from `///` comment blocks near symbols.
Other doc styles are not extracted by this adapter path.

## 4) Discovery hard limits are fixed

`src/docgen/core.rs` sets:

- max file size: 2 MB
- max files: 20,000
- max depth: 64

These are good safety defaults, but can silently omit data in very large repos unless surfaced clearly in diagnostics.

## 5) Link checking is intentionally narrow

In `src/docgen/gaps.rs`, broken-link checks skip `http://`, `https://`, and `mailto:` links.
It checks local existence only and does not verify external availability or anchors.

## 6) Builtins can dominate counts

By default, builtins are included unless `--no-builtins` is set.
For repo-quality reporting, builtin symbol volume can dilute signal from project-defined symbols.

## Best Next Steps (Prioritized)

## P0: Improve signal quality for CI gating

1. Add a CI docsgen mode for repositories using:
   - `--public-only`
   - `--no-builtins`
   - `--fail-on-undocumented`
2. Keep developer preview mode (`--include-private`) for local insight, but do not gate on it.

Expected result: gate failures map to true exported API gaps, not internal helpers.

## P1: Strengthen Kujo visibility semantics

1. Update Kujo adapter visibility heuristics to avoid auto-public for all top-level `func`.
2. Align with Kujo language visibility rules (prefer explicit `pub` or module export signals).
3. Add regression tests in `tests/docgen_universal.rs` for private top-level helper patterns.

Expected result: fewer false-positive undocumented public symbols.

## P1: Expand Kujo doc comment extraction

1. Support additional Kujo doc comment forms if the language allows them.
2. Keep `///` support, but extend parsing rules and tests.

Expected result: better doc attachment coverage without changing source code style immediately.

## P2: Improve diagnostics for discovery truncation

1. Emit explicit diagnostics when files are skipped due to `max_file_size_bytes`, `max_files`, or `max_depth`.
2. Include skip counts in JSON summary so CI can monitor omission drift.

Expected result: easier trust and troubleshooting for large repos.

## P2: Enhance link validation modes

1. Keep current local-only link check as default (safe/fast).
2. Add optional strict modes:
   - anchor validation for local docs links
   - optional external URL reachability check with timeout and allowlist

Expected result: stronger doc quality checks when needed, without slowing normal runs.

## P3: Improve summary ergonomics

1. Include `project_symbol_count` separate from `builtin_symbol_count` in CLI JSON summary.
2. Include per-kind counts (function/method/struct/enum/etc.) in summary.

Expected result: easier quality dashboards and more actionable CI output.

## Suggested Immediate CI Profile

Use this command as a baseline quality gate:

```bash
kujo docgen . \
  --languages kujo \
  --public-only \
  --no-builtins \
  --fail-on-undocumented \
  --fail-on-broken-links \
  --fail-on-warnings \
  --out-dir docs/generated
```

And keep this developer audit command for improvement work:

```bash
kujo docgen . \
  --languages kujo \
  --include-private \
  --emit-ai-tasks \
  --search-index \
  --out-dir docs/generated-dev
```

## Final Assessment

`docsgen` works correctly for generation and contract stability across the three repos tested.
Current limitations are mainly around extraction depth and visibility heuristics, not runtime reliability.
Applying the P0/P1 steps above should significantly reduce false positives and make CI gating practical.

## QA Hardening Task DG-QA-010 follow-up (2026-05-19)

After implementing shared visibility-policy helpers across adapters (with Kujo and TypeScript visibility semantics regression-locked), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: `DG-QA-010` is a maintainability refactor with visibility-regression coverage hardening; strict external gate metrics remain unchanged while visibility-policy logic is now centralized.

## QA Hardening Task DG-QA-011 follow-up (2026-05-19)

After moving DocGen CLI JSON contract assembly to a typed single-source payload builder (with dedicated snapshot contract coverage), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: `DG-QA-011` is contract-serialization and test-hardening work; strict external gate counts remain stable while JSON contract assembly is now centralized and snapshot-locked.

## QA Hardening Task DG-QA-012 follow-up (2026-05-19)

After renderer deduplication cleanup (removing no-op duplicate branches and centralizing shared source-location formatting helpers across renderers), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: `DG-QA-012` is renderer maintainability and regression-hardening work; strict external gate counts remain unchanged.

## QA Hardening Task DG-QA-013 follow-up (2026-05-19)

After adding configurable discovery-limit overrides from CLI/env and emitting effective limits in machine-readable DocGen summaries, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: `DG-QA-013` is configuration/contract hardening; strict external gate counts remain unchanged while effective discovery budgets are now explicit (`discovery_limits`) for reproducible CI runs.

## QA Hardening Task DG-QA-014 follow-up (2026-05-19)

After adding per-language adapter health counters and low-yield extraction diagnostics, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: `DG-QA-014` is observability hardening; strict external gate counts remain unchanged while adapter extraction health is now machine-readable and low-yield cases emit deterministic diagnostics.

## QA Hardening Task DG-QA-015 follow-up (2026-05-19)

After adding optional incremental cache mode (`--cache-dir`) and JSON cache hit/miss counters, strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: `DG-QA-015` adds CI-scale incremental reuse and machine-visible cache counters while preserving strict external gate stability.

## QA Hardening Task DG-QA-016 follow-up (2026-05-19)

After adding source-link provider abstraction and template-driven source URL rendering (with path-normalization safety guards), strict-mode metrics for the same repos are:

| Repo | undocumented_count (before) | undocumented_count (after) | delta | broken_link_count delta | warning_count delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `ai-sdk` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `mcp` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |
| `scout` | 0 | 0 | 0 | 0 -> 0 (0) | 0 -> 0 (0) |

Interpretation: `DG-QA-016` is rendering/configuration hardening; strict external gate counts remain stable while source-link provider behavior is now configurable and safety-guarded.
