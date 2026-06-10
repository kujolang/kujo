# V1H-SIZE-002 — Feature Gate Matrix And Size Deltas

Date: 2026-05-24
Item: `V1H-SIZE-002`

## Summary

Implemented additive, default-compatible feature gates for heavyweight runtime subsystems and validated reduced build profiles.

Feature gates added:
- `runtime-db`
- `runtime-image`
- `runtime-archive`
- `runtime-jit`
- `default = ["runtime-db", "runtime-image", "runtime-archive", "runtime-jit"]`

Default builds preserve existing behavior. Reduced builds now compile with deterministic disabled-feature runtime diagnostics for gated native surfaces.

## Implementation artifacts

- `Cargo.toml`: feature matrix declaration.
- `src/lib.rs` and `src/main.rs`: JIT module selection by feature (`runtime-jit`) with `jit_disabled.rs` fallback.
- `src/jit_disabled.rs`: no-op JIT compatibility surface for non-JIT builds.
- `src/interpreter/native_functions/mod.rs`: DB feature-disabled dispatch error surface.
- `src/interpreter/native_functions/filesystem.rs`: image/archive feature-disabled dispatch error surfaces.

## Build matrix commands

- `cargo check`
- `cargo check --no-default-features`
- `cargo check --no-default-features --features runtime-jit`
- `cargo check --no-default-features --features runtime-db,runtime-image,runtime-archive`

All commands completed successfully.

## Size measurements

Commands:
- `cargo build --release`
- `cargo build --release --no-default-features`
- `cargo build --release --no-default-features --features runtime-jit`
- `cargo build --release --no-default-features --features runtime-db,runtime-image,runtime-archive`

Byte sizes:
- default: `24099968`
- no-default-features: `19645964`
- no-default-features + runtime-jit: `21872272`
- no-default-features + runtime-db,runtime-image,runtime-archive: `21877780`

Interpretation:
- Reduced profile (`--no-default-features`) is ~4.45 MB smaller than default in this environment.
- JIT-only and DB/image/archive-only subsets sit between the two extremes and provide reproducible size-control knobs.

## Tests and validation

- `cargo test --test binary_size_baseline_contract` (3 passed)
- `cargo test --lib release_hardening_database_module_dispatch_argument_contracts` (1 passed)
- `cargo test --lib release_hardening_load_image_dispatch_contracts` (1 passed)
- `cargo test --lib release_hardening_zip_module_dispatch_argument_contracts` (1 passed)
- `cargo test --test vm_interpreter_parity_surfaces` (87 passed)

Full-suite note:
- `cargo test` was run; known pre-existing failure persists in `tests/docs_examples.rs` for `docs/NATIVE_API_SECURITY_POSTURE.md#1` snippet parse mismatch (unrelated to this change).
