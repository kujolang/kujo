# V1H-SIZE-003 — Shared HTTP Path/Query Helper Dedup

Date: 2026-05-22  
Checklist item: `V1H-SIZE-003`

## Objective

Reduce DRY duplication by consolidating identical HTTP path/query parsing helper logic used in both interpreter and VM runtime server flows.

## Implementation

1. Added shared module `src/http_request_utils.rs`:
   - `split_http_path_and_query(url: &str) -> (String, HashMap<String, String>, String)`
   - internal lexical query parsing behavior identical to previous runtime-local implementations
2. Added helper unit tests:
   - no-query path behavior
   - multi-pair query parsing without URL decoding
   - empty-key filtering and missing-value behavior
3. Removed duplicate helper implementations from:
   - `src/interpreter/mod.rs`
   - `src/vm.rs`
4. Rewired both runtime paths to call:
   - `http_request_utils::split_http_path_and_query(...)`

## Commands Run

```bash
cargo test split_http_path_and_query
cargo test vm_http_server_route_method_returns_updated_server
cargo test vm_http_handler_wrapper_executes_lambda_response_correctly
cargo test --test vm_interpreter_parity_surfaces
```

## Results

- `split_http_path_and_query` helper tests: pass (lib/main test binaries)
- VM HTTP route method test: pass
- VM HTTP handler wrapper test: pass
- VM/interpreter parity suite: pass (`86 passed, 0 failed`)

## Outcome

Duplicated runtime helper logic was removed without changing HTTP path/query behavior contracts, and parity/runtime tests remained green.
