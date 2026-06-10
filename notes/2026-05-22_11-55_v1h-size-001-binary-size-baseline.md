# V1H-SIZE-001 — Binary Size Baseline Matrix

Date: 2026-05-22  
Checklist item: `V1H-SIZE-001`

## Objective

Create a reproducible binary-size measurement path with host/toolchain metadata and debug/release/stripped byte counts.

## Implementation

1. Added `scripts/measure_binary_size.sh`:
   - prints deterministic host/toolchain metadata
   - supports `--dry-run` and `--metadata-only`
   - builds `target/debug/kujo` and `target/release/kujo`
   - reports byte sizes and stripped-release size when `strip` is available
2. Added contract coverage in `tests/binary_size_baseline_contract.rs`:
   - help contract
   - dry-run command emission contract
   - unknown-argument failure-path contract

## Commands Run

```bash
cargo test --test binary_size_baseline_contract
bash scripts/measure_binary_size.sh
```

## Results

- `binary_size_baseline_contract`: pass (`3 passed, 0 failed`)
- `measure_binary_size.sh` output:
  - `debug`: `91597784` bytes
  - `release`: `31006832` bytes
  - `release_stripped`: `26557120` bytes
- Metadata captured by script:
  - host: `x86_64-apple-darwin`
  - rustc: `1.86.0`
  - cargo: `1.86.0`

## Outcome

Binary-size baseline collection is now repeatable through a committed script and contract-tested CLI interface.
