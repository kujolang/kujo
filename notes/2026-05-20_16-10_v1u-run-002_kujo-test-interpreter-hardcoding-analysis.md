# V1U-RUN-002 — `kujo test` Interpreter Hardcoding Analysis

Date: 2026-05-20

## Scope

Explain and justify why `src/parser.rs::run_all_tests` still hardcodes
`kujo run <fixture> --interpreter`, and define removal criteria.

## Evidence Collection

Command used:

```bash
bash -lc 'set -euo pipefail
count=0
mismatch=0
for file in tests/*.kujo; do
  count=$((count+1))
  vm_out=$(cargo run --quiet -- run "$file" 2>&1 || true)
  interp_out=$(cargo run --quiet -- run "$file" --interpreter 2>&1 || true)
  if [[ "$vm_out" != "$interp_out" ]]; then
    mismatch=$((mismatch+1))
    echo "MISMATCH:$file"
    if [[ $mismatch -ge 15 ]]; then
      break
    fi
  fi
  if [[ $count -ge 120 ]]; then
    break
  fi
done

echo "SCANNED=$count MISMATCHES=$mismatch"'
```

Observed summary:

- `SCANNED=21 MISMATCHES=15`
- Example divergent fixtures:
  - `tests/array_methods_test.kujo`
  - `tests/net_test.kujo`
  - `tests/error_call_stack_test.kujo`
  - `tests/image_processing_test.kujo`
  - `tests/jit_inline_cache.kujo`

Observed divergence classes:

1. Diagnostic-shape drift: VM vs interpreter runtime error identifiers and subsystem markers (`[RUFVM001]` vs `[RUFRUN001]`).
2. VM-specific optimization banners/noise appearing in output streams for some fixtures.
3. Legacy fixture behavior depending on runtime-path-specific builtin availability/behavior.

## Decision

Keep `kujo test` interpreter-pinned temporarily.

Rationale: the fixture corpus and snapshots are still coupled to interpreter-first behavior,
and measured runtime-path drift remains large enough that blindly switching to VM-first would
produce broad output churn and ambiguous failures.

## Removal Criteria (tracked under `V1U-RUN-003`)

1. Implement an explicit runtime-path strategy for `kujo test` (VM-first or dual-engine) with deterministic fallback policy.
2. Normalize or rebaseline snapshot expectations for runtime-path-specific diagnostics/noise.
3. Add parity/contract coverage for currently divergent fixture classes and prove the chosen strategy with command-level tests.
