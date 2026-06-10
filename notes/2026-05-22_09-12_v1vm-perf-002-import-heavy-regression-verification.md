# V1VM-PERF-002 — Import-Heavy Perf Regression Verification

Date: 2026-05-22
Owner: codex agent

## Scope

Compared import-heavy nested-module startup benchmark performance before/after the reliability-fix sequence and applied a deterministic tolerance rule.

## Benchmark command

```bash
cargo bench --bench v1_perf_benchmarks -- import_heavy_nested_dotted_startup_cold_loader --noplot --sample-size 10 --warm-up-time 0.5 --measurement-time 1
```

## Comparison

- Baseline (`V1VM-PERF-001`) median: `350.61 ms`
- Current median: `40.763 ms`
- Median delta: `-88.37%`

## Tolerance policy and verdict

- Threshold: unacceptable regression if current median > baseline median by `20.0%`.
- Verdict: `PASS` (no unacceptable regression).

## Artifact and contract

- Artifact: `docs/generated/VM_IMPORT_HEAVY_PERF_COMPARISON.md`
- Contract: `tests/vm_import_heavy_perf_comparison_contract.rs`
