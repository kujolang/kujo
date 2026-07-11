# Candidate catalog

Scores are 1–5. Maintenance cost is scored positively (5 = low burden). The
total is directional; status and evidence quality control the recommendation.

| ID | Candidate | Category | Layer | Spread | Duplication | Safety | Stability | Agent | Total | Confidence | Status |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---|---|
| HLP-001 | atomic bounded file write | filesystem | runtime/stdlib | 5 | 5 | 5 | 4 | 5 | 42/50 | high | implemented |
| HLP-002 | symlink-aware path boundary | filesystem/security | runtime/stdlib | 4 | 4 | 5 | 3 | 5 | 39/50 | medium | prototype |
| HLP-003 | typed structured-data access | JSON/types | language/stdlib design | 5 | 5 | 4 | 3 | 5 | 39/50 | medium | design further |
| HLP-004 | declarative CLI token parser | CLI | first-party package | 4 | 4 | 3 | 4 | 4 | 37/50 | high | implemented spike |
| HLP-005 | redaction profiles and contract | security | official package | 4 | 4 | 5 | 3 | 5 | 38/50 | medium | centralize outside core |
| HLP-006 | recursive ensure-directory | filesystem | package/docs | 4 | 4 | 3 | 4 | 4 | 35/50 | medium | prototype |
| HLP-007 | canonical ISO/slug/padding examples | text/time | documentation | 4 | 3 | 2 | 5 | 3 | 34/50 | high | documented |
| HLP-008 | bounded file read | filesystem/agent safety | runtime/stdlib | 3 | 3 | 5 | 4 | 5 | 36/50 | medium | prototype |
| HLP-009 | bounded recursive file walk | filesystem | package/runtime | 4 | 3 | 4 | 3 | 4 | 34/50 | medium | prototype |
| HLP-010 | JSON file read/write envelope | structured data | first-party package | 3 | 4 | 3 | 3 | 4 | 33/50 | medium | centralize outside core |
| HLP-011 | typed environment schema | config | package/docs | 3 | 3 | 3 | 4 | 4 | 32/50 | high | documented |
| HLP-012 | project-root/config discovery | workspace | internal tooling package | 3 | 3 | 3 | 3 | 4 | 30/50 | medium | centralize outside core |
| HLP-013 | process-result accessors | process | documentation | 3 | 3 | 3 | 4 | 4 | 32/50 | high | documented |
| HLP-014 | retry/backoff policy object | networking/AI | official package | 3 | 3 | 3 | 3 | 4 | 30/50 | medium | prototype |
| HLP-015 | stable canonical JSON helper | serialization | documentation/runtime | 3 | 2 | 3 | 5 | 4 | 32/50 | high | document existing API |
| HLP-016 | shared test workspace helpers | testing | test-support package | 3 | 3 | 2 | 3 | 3 | 26/50 | low | centralize outside core |
| HLP-017 | structured error/result conventions | errors | language/package design | 4 | 3 | 4 | 3 | 4 | 32/50 | medium | design further |
| HLP-018 | `group_by`/`index_by` collections | collections | stdlib | 2 | 2 | 2 | 2 | 3 | 24/50 | low | defer |
| HLP-019 | pagination/HTTP policy bundle | networking | domain package | 2 | 3 | 2 | 2 | 3 | 22/50 | low | reject for core |
| HLP-020 | shell quoting helper | process/security | none | 3 | 4 | 1 | 1 | 1 | 17/50 | high | reject |
| HLP-021 | pluralization/domain text | strings | domain package | 1 | 2 | 1 | 1 | 1 | 15/50 | high | reject |
| HLP-022 | broad convenience aliases | all | core | 5 | 3 | 1 | 1 | 1 | 18/50 | high | reject |

The five detailed proposals are HLP-001 through HLP-005. HLP-006 through
HLP-022 are fully dispositioned in the roadmap and negative-findings sections.
