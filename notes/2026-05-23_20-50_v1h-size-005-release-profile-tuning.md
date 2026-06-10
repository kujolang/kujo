# V1H-SIZE-005 - Release Profile Tuning

Date: 2026-05-23
Item: V1H-SIZE-005

## Summary

Added additive release-profile tuning in `Cargo.toml` to reduce binary size without removing features:

- `[profile.release] lto = "thin"`
- `[profile.release] codegen-units = 1`
- `[profile.release] strip = "symbols"`

## Before/After Size Evidence

Before baseline (from `notes/2026-05-22_11-55_v1h-size-001-binary-size-baseline.md`):

- debug: `91597784`
- release: `31006832`
- release_stripped: `26557120`

After tuning (this loop):

```bash
cargo build --release
wc -c target/debug/kujo target/release/kujo
cp target/release/kujo target/release/kujo.stripped && strip target/release/kujo.stripped && wc -c target/release/kujo.stripped
```

Observed:

- debug: `91593328`
- release: `24067240`
- release_stripped-copy: `24067320`

## Validation

- `cargo test` -> PASS (full suite; no failures)

## Notes

- Two pre-change/post-change script attempts were impacted by temporary Cargo build-lock contention from concurrent agent activity; explicit reproducible release-size commands above were used to capture final post-change evidence deterministically.
