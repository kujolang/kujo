# Performance Hot-Path Audit (2026-05-26)

Scope: `ER-P1-002`

## Covered hot paths

Bench/harness coverage is sourced from `benches/v1_perf_benchmarks.rs` and generated perf artifacts:

- VM/interpreter runtime dispatch workload:
  - `vm/loops_calls_recursion_strings_collections`
  - `interpreter/loops_calls_recursion_strings_collections`
- Module loading/import-heavy startup:
  - `module_resolution/import_heavy_nested_dotted_startup_cold_loader`
  - `module_resolution/import_heavy_nested_dotted_cached_lookup_warm_loader`
- Parser/lexer throughput:
  - `parser/large_file`, `parser/deep_expression`
  - `lexer/large_source`, `lexer/many_tokens`

## Current benchmark evidence (committed artifacts)

- `docs/generated/VM_IMPORT_HEAVY_CACHE_LOOKUP.md`
  - warm cached lookup median: `251.22 µs`
  - cold startup median: `35.467 ms`
  - interpreted cache effect: approximately `141x` improvement on warm path.
- `docs/generated/VM_IMPORT_HEAVY_PERF_COMPARISON.md`
  - baseline median: `350.61 ms`
  - current median: `40.763 ms`
  - delta: `-88.37%` (`PASS` against `20%` regression threshold).

## Validation commands

```bash
cargo test --test vm_import_heavy_cache_lookup_contract
cargo test --test vm_import_heavy_perf_comparison_contract
```

Both contract suites passed on 2026-05-26.

## Regression status

- No import-heavy startup/cache regressions are indicated by committed perf artifacts.
- No additional high-impact runtime perf regressions were observed in this audit pass.

## Owner and timeline

- Owner: runtime-owner
- Follow-up cadence: re-run targeted `cargo bench --bench v1_perf_benchmarks -- import_heavy_nested_dotted_ ...` in next scheduled perf sweep window and refresh generated artifacts when material runtime/path changes land.
