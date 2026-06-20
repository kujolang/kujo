# Kujo Performance Guide

This guide describes Kujo's release-candidate performance posture. It avoids
public speed claims unless they are backed by committed benchmark artifacts or
repeatable commands in this repository.

## Runtime Posture

Kujo is VM-first for normal execution:

```bash
kujo run script.kujo
```

The tree-walking interpreter remains available as an explicit compatibility and
debug path:

```bash
kujo run --interpreter script.kujo
```

The JIT is experimental and opt-in on JIT-compatible bytecode surfaces:

```bash
kujo run --jit script.kujo
```

`--jit` is not the default runtime mode. When `--jit` is requested for a program
that contains an unsupported bytecode surface, Kujo emits deterministic fallback
messaging and executes the program on the VM without JIT. The CLI contract is
covered by `tests/jit_execution_contract.rs`.

The crate feature `runtime-jit` controls whether the JIT implementation is
compiled into the binary. It is enabled by the default feature set, and
`cargo check --no-default-features --features runtime-jit` is the reduced build
that checks JIT-only feature wiring.

## Current Benchmark Evidence

Current launch-safe benchmark evidence is committed as generated artifacts and
audit notes:

- `docs/PERF_HOT_PATH_AUDIT_2026-05-26.md`
- `docs/generated/VM_IMPORT_HEAVY_PERF_COMPARISON.md`
- `docs/generated/VM_IMPORT_HEAVY_CACHE_LOOKUP.md`

The committed import-heavy module-resolution evidence reports:

| Evidence | Result |
| --- | --- |
| Cold nested dotted startup comparison | baseline median `350.61 ms`; current median `40.763 ms`; median delta `-88.37%`; `PASS` against the documented 20% regression threshold |
| Warm cached lookup | warm median `251.22 us`; cold median `35.467 ms`; approximately `141x` warm-cache improvement |

These numbers are regression evidence for the named benchmark workloads, not a
general claim that all Kujo programs have the same speedup.

## Reproducing Benchmark Evidence

Run the committed perf contract tests:

```bash
cargo test --test vm_import_heavy_perf_comparison_contract
cargo test --test vm_import_heavy_cache_lookup_contract
```

Re-run the Criterion benchmark family when runtime or module-loading changes
materially affect these paths:

```bash
cargo bench --bench v1_perf_benchmarks -- import_heavy_nested_dotted_ --noplot --sample-size 10 --warm-up-time 0.5 --measurement-time 1
```

Refresh generated artifacts only after inspecting the new benchmark output and
confirming the method and environment are comparable.

## Profiling Tools

Kujo includes profiling commands for local investigation:

```bash
kujo profile script.kujo
kujo profile script.kujo --cpu
kujo profile script.kujo --memory
kujo profile script.kujo --jit
kujo profile script.kujo --flamegraph profile.txt
```

The profiler output can include CPU, memory, and JIT statistics sections. Treat
JIT statistics as instrumentation for experimental opt-in runs, not as evidence
that default `kujo run` used JIT.

## Benchmarking Commands

Kujo includes a benchmarking command for local scripts:

```bash
kujo bench examples/benchmarks/
kujo bench fibonacci.kujo
kujo bench fibonacci.kujo -i 20 -w 5
```

Benchmark output is useful for local comparison and regression diagnosis. Do not
publish broad cross-language or JIT speedup claims from ad hoc runs; use a
documented benchmark campaign with pinned hardware, commands, inputs, and
artifacts.

## Optimization Guidance

- Use default VM execution for production-like script runs.
- Use `--interpreter` when isolating interpreter/VM differences or debugging.
- Use `--jit` only for experiments or workloads that are known to stay within
  the currently supported JIT surface.
- Keep hot loops type-stable when testing JIT behavior.
- Prefer native helpers for filesystem, collection, crypto, archive, image, and
  database work when they match the task.
- Profile before optimizing; do not infer bottlenecks from source shape alone.

## Troubleshooting

### Program is slower than expected

Run:

```bash
kujo profile script.kujo
```

Check whether the workload is CPU-bound, I/O-bound, allocation-heavy, or using
the interpreter path.

### JIT was requested but did not run

Check for the fallback message:

```text
JIT opt-in requested, but this program is not JIT-compatible (...). Falling back to VM bytecode execution without JIT.
```

That message means the program completed on the VM path after an explicit JIT
request encountered an unsupported surface.

### Feature-reduced build behavior is unclear

Use the feature-matrix checks tracked by release readiness:

```bash
cargo check --no-default-features
cargo check --no-default-features --features runtime-jit
cargo check --no-default-features --features runtime-db,runtime-image,runtime-archive
```

## Related Docs

- `README.md` for runtime mode recommendations.
- `docs/VM_INTERPRETER_PARITY_MATRIX.md` for VM/interpreter parity and the
  current JIT boundary statement.
- `docs/RELEASE_PROCESS.md` for release-gate and optional benchmark smoke
  commands.
- `docs/HETZNER_BENCHMARK_SETUP_AND_PRICING.md` and
  `docs/SSG_BENCHMARK_NEXT_STEPS.md` for future publishable benchmark campaign
  planning.

## LSP Latency Guardrails

Kujo tracks conservative latency guardrails for key editor-loop operations using
representative source samples:

- completion (`textDocument/completion` equivalent helper path)
- diagnostics (`textDocument/publishDiagnostics` equivalent helper path)
- hover (`textDocument/hover` equivalent helper path)

Guardrail test:

```bash
cargo test --test lsp_latency_guardrails
```

Related reliability tests:

```bash
cargo test lsp_server::tests::cancelled_request_returns_cancelled_error
cargo test lsp_server::tests::timeout_returns_timeout_error_shape
cargo test lsp_server::tests::non_object_json_message_does_not_panic_or_emit_response
```

Last updated: 2026-06-19 for v1.0 release-readiness posture.
