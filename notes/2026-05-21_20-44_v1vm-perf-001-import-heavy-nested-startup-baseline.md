# V1VM-PERF-001 — Import-Heavy Nested Startup Benchmark Baseline

Date: 2026-05-21
Owner: codex agent

## Scope implemented

Added a dedicated Criterion benchmark for nested dotted-module startup on the interpreter-backed module loader path:

- Benchmark id: `module_resolution/import_heavy_nested_dotted_startup_cold_loader`
- File: `benches/v1_perf_benchmarks.rs`
- Fixture shape:
  - Creates 64 nested modules under `src/core/mod_*.kujo`
  - Each module exports a unique symbol (`value_000`, `value_001`, ...)
  - Entry module imports all via dotted `from src.core.mod_* import value_*`
- Execution model:
  - Fresh/cold `ModuleLoader` per iteration
  - `load_module(entry_module)` triggers parser + interpreter-backed module evaluation on nested dotted imports

## Command used

```bash
cargo bench --bench v1_perf_benchmarks -- import_heavy_nested_dotted_startup_cold_loader --noplot --sample-size 10 --warm-up-time 0.5 --measurement-time 1
```

## Baseline captured

- `time: [278.40 ms 350.61 ms 420.40 ms]`
- Criterion warning observed:
  - Unable to complete 10 samples in 1.0s; estimated collection around 2.15s.

This is accepted as the initial baseline for `V1VM-PERF-001`; follow-on comparison/tolerance work belongs in `V1VM-PERF-002`.
