# CLI Dual-Mode Output Inventory

Status: active  
Last updated: 2026-05-31

This inventory records CLI surfaces that expose both machine-readable and human-readable output modes, plus their stability expectations for automation and operator tooling.

## Scope

Dual-mode in this document means one or more of:

- explicit `--json` + plain-text modes
- JSON error envelope mode paired with default stderr diagnostics mode

## Inventory

| Command | Machine-readable mode | Human mode | Error routing | Stability class |
| --- | --- | --- | --- | --- |
| `kujo check` | `--json` object payload (`command`, `status`, counters) | concise pass/fail summary (`--quiet`/`--verbose` variants) | non-zero exits write diagnostics to stderr | JSON schema: stable. Human text: stable by convention, not schema-locked. |
| `kujo format` | `--json` object payload (status/options/formatted source preview) | plain formatter summary/status lines | non-zero exits write diagnostics to stderr | JSON schema: stable. Human text: stable by convention. |
| `kujo lint` | `--json` array of issue objects | plain lint diagnostics lines | non-zero exits write diagnostics to stderr | JSON schema: stable. Human text: stable by convention. |
| `kujo docgen` | `--json` object payload (paths/counts/gates/summary) | plain run summary and diagnostics | non-zero exits write diagnostics to stderr | JSON schema: stable. Human text: stable by convention. |
| `kujo run` | `--json-runtime-diagnostics` error envelope on runtime failures | default runtime/diagnostic stderr text | JSON mode runtime failures: stdout JSON + non-zero exit. Default mode: stderr diagnostics + non-zero exit. | JSON envelope: stable. Default stderr text: stable by convention. |
| `kujo lsp-complete` | `--json` array of `{label, kind}` | tab-delimited `label<TAB>kind` rows | read/parse/runtime failures use non-zero exit + stderr | JSON schema: stable. Plain row shape: stable by contract test. |
| `kujo lsp-definition` | `--json` object or `null` | `name<TAB>file:line:column` or `not found` | read/parse/runtime failures use non-zero exit + stderr | JSON schema: stable. Plain row shape: stable by convention. |
| `kujo lsp-references` | `--json` array of `{line,column,is_definition}` | tab-delimited role + location rows | read/parse/runtime failures use non-zero exit + stderr | JSON schema: stable. Plain row shape: stable by convention. |
| `kujo lsp-hover` | `--json` object or `null` | tab-delimited symbol/detail/location or `not found` | read/parse/runtime failures use non-zero exit + stderr | JSON schema: stable. Plain row shape: stable by convention. |
| `kujo lsp-diagnostics` | `--json` array of diagnostic objects | tab-delimited diagnostic rows | read/parse/runtime failures use non-zero exit + stderr | JSON schema: stable. Plain row shape: stable by convention. |
| `kujo lsp-rename` | `--json` object with edits and updated source | summary row + per-edit tab-delimited rows | rename validation/runtime failures use non-zero exit + stderr | JSON schema: stable. Plain row shape: stable by convention. |
| `kujo lsp-code-actions` | `--json` array of action objects | tab-delimited action rows | read/parse/runtime failures use non-zero exit + stderr | JSON schema: stable. Plain row shape: stable by convention. |

## Intentional Instability Surface

These surfaces are intentionally treated as non-schema operator output and may evolve for readability:

- section headers and visual separators in benchmark/profile/repl human output
- warning phrasing where semantics are unchanged
- non-JSON human formatting details outside contract tests

Stability guarantee still applies to:

- exit-code policy
- JSON field presence and types for documented machine-readable modes
- explicit row-shape constraints covered by CLI contract tests

## Test Coverage Anchors

- JSON schema contracts: `tests/cli_json_contracts.rs`
- LSP plain/json mode and stderr routing contracts: `tests/cli_contracts.rs`
- runtime diagnostics envelope contract: `tests/cli_json_contracts.rs::run_runtime_json_diagnostic_contract_is_stable`

## Snapshot Update Discipline

Human-render snapshot-like assertions should only be updated when the output intent changes deliberately.
For readability-only changes, include:

1. a brief reason in the commit message
2. updated test assertions in the same change
3. an explicit note that exit-code and JSON contracts are unchanged
