# V1X-SIZE-001 - Optional Runtime Dependencies And Size Evidence

Date: 2026-05-27
Owner: AI agent loop execution

## Summary

Completed additive Cargo feature/dependency wiring so heavy runtime stacks are wired as true optional dependencies while preserving default behavior.

Updated in `Cargo.toml`:

- `runtime-db = ["dep:rusqlite", "dep:postgres", "dep:mysql_async"]`
- `runtime-image = ["dep:image"]`
- `runtime-archive = ["dep:zip"]`
- `rusqlite`, `postgres`, `mysql_async`, `image`, `zip` marked `optional = true`

Default feature set remains unchanged:

- `default = ["runtime-db", "runtime-image", "runtime-archive", "runtime-jit"]`

## Binary Size Evidence

### Pre-change baseline

Command:

```bash
scripts/measure_binary_size.sh
```

Result:

- debug: `91828056` bytes
- release: `24157416` bytes
- release_stripped: `24157496` bytes

### Post-change baseline

Command:

```bash
scripts/measure_binary_size.sh
```

Result:

- debug: `91828056` bytes
- release: `24157416` bytes
- release_stripped: `24157496` bytes

### Reduced profile (JIT disabled at build)

Command:

```bash
cargo build --release --no-default-features --features runtime-db,runtime-image,runtime-archive
wc -c target/release/kujo
```

Result:

- release (no JIT): `21955772` bytes
- delta vs default release: `-2201644` bytes (`-9.11%`)

## Validation Commands And Results

```bash
cargo build --release
```

- PASS

```bash
cargo test --test cli_contracts
```

- PASS (`15 passed; 0 failed`)

```bash
cargo test --test vm_interpreter_parity_surfaces
```

- PASS (`100 passed; 0 failed`)

```bash
cargo test
```

- PARTIAL BLOCKER (environment/tooling): failed in `tests/interpreter_flag_dependency_map_contract.rs` because `scripts/generate_interpreter_flag_dependency_map.sh` requires `rg`, which is not available in this environment (`rg: command not found`).
- This failure is unrelated to `V1X-SIZE-001` Cargo feature wiring change.

## Notes

- Working tree is globally dirty from parallel-agent activity; only `V1X-SIZE-001`-scoped files were changed/staged for this loop.
- Release/tag/publish actions were not executed.
