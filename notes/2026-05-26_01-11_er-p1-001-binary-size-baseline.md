# ER-P1-001 — Binary size baseline and optimization evidence

Date: 2026-05-26
Item: ER-P1-001

## Summary

Closed ER-P1-001 with reproducible release-binary size measurements and regression checks.

## Measurement outputs

- Baseline profile override (`strip=none`, `lto=off`, `codegen-units=16`):
  - `/tmp/kujo_size_base/release/kujo` -> `34,666,816` bytes (33.1 MiB)
- Current optimized release defaults (`strip=symbols`, `lto=thin`, `codegen-units=1`):
  - `/tmp/kujo_release_optimized` -> `24,149,152` bytes (23.0 MiB)
- Delta:
  - `-10,517,664` bytes (`-30.34%`)

## Validation

- `cargo test --test cli_contracts` -> PASS (15/15)
- `cargo test --test vm_interpreter_parity_surfaces` -> PASS (100/100)

## Tradeoff note

- `strip=symbols` reduces binary size materially but removes symbol metadata useful for low-level debugging.
- `thin` LTO + `codegen-units=1` increase release compile time while improving size posture and typically preserving or slightly improving runtime performance.
