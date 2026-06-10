# Dead Code Allow Hotspot Audit (V1H-SIZE-004)

Date: 2026-05-22
Owner: Codex agent loop execution

## Scope

Inventory and classify `#[allow(dead_code)]` hotspots, with a low-risk removal/gating pass where behavior remains unchanged.

## Inventory Snapshot

Command:

```bash
rg "#\[allow\(dead_code\)\]" -n src | cut -d: -f1 | sort | uniq -c | sort -nr
```

Top hotspots observed:

| File | Count | Classification | Rationale |
| --- | ---: | --- | --- |
| `src/builtins.rs` | 40 | Keep (defer) | Builtin surface exports a broad native API; many functions are intentionally externally reachable even if not always referenced by current call graphs. Candidate for future feature-gating (`V1H-SIZE-002`). |
| `src/interpreter/value.rs` | 31 | Keep (defer) | Runtime value model includes forward-compatible variants/helpers needed for parser/runtime parity and tests; removing now risks semantic churn. |
| `src/jit.rs` | 22 | Keep (defer) | JIT integration scaffolding and profiling infrastructure intentionally staged; overlaps with unsafe/invariant work in `V1H-UNSAFE-*`. |
| `src/ast.rs` | 13 | Keep (defer) | AST variants are consumed across parser/resolver/test tooling; static dead-code warnings do not imply removable language surface. |
| `src/vm.rs` | 5 | Keep (defer) | VM-first migration includes deferred closure/upvalue and integration paths documented in scope docs; not safe for ad hoc removal. |
| `src/module.rs` | 2 | Keep (defer) | Module loader structs expose fields used by module cache/exports and diagnostics flows; low direct size impact. |
| `src/path_security.rs` | 1 | Remove/gate now | `reject_url_encoded_parent_traversal` was only used by unit tests. Moved behind `#[cfg(test)]` and dropped runtime `#[allow(dead_code)]`. |

## Applied change (low risk)

- Converted `reject_url_encoded_parent_traversal` in `src/path_security.rs` from always-compiled API with `#[allow(dead_code)]` to test-only (`#[cfg(test)] pub(crate)`), preserving test coverage and removing production dead-code allowance.

## Deferred work

- Feature-gating of heavyweight optional subsystems remains tracked by `V1H-SIZE-002`.
- Broad dead-code cleanup in JIT/VM/builtins remains intentionally deferred to avoid behavior or compatibility regressions before parity hardening completes.
