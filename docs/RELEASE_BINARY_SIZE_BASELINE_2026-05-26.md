# Release Binary Size Baseline (2026-05-26)

Scope: `ER-P1-001`

## Measurement commands

Optimized profile (current repo defaults in `[profile.release]`):

```bash
cargo build --release
cp target/release/kujo /tmp/kujo_release_optimized
stat -f "%z" /tmp/kujo_release_optimized
```

Baseline profile (override to emulate non-optimized release profile):

```bash
CARGO_PROFILE_RELEASE_STRIP=none \
CARGO_PROFILE_RELEASE_LTO=off \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
CARGO_TARGET_DIR=/tmp/kujo_size_base \
cargo build --release
stat -f "%z" /tmp/kujo_size_base/release/kujo
```

## Results

| Profile | Bytes | Approx Size | Delta vs Baseline |
| --- | ---: | ---: | ---: |
| Baseline overrides (`strip=none`, `lto=off`, `codegen-units=16`) | 34,666,816 | 33.1 MiB | n/a |
| Current optimized release defaults (`strip=symbols`, `lto=thin`, `codegen-units=1`) | 24,149,152 | 23.0 MiB | -10,517,664 bytes (-30.34%) |

## Tradeoffs

- `strip = "symbols"` significantly reduces binary size, but removes symbol metadata helpful for low-level debugging.
- `lto = "thin"` and `codegen-units = 1` increase release build time, but commonly improve size and can preserve or slightly improve runtime performance.
- No CLI/runtime contract regressions were observed in focused validation for this pass.

## Validation

```bash
cargo test --test cli_contracts
cargo test --test vm_interpreter_parity_surfaces
```
