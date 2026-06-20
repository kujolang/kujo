# V1 Clean Feature Matrix Revalidation - 2026-06-19

## Scope

`V1RR-P1-004` required a clean checkout/worktree release build, reduced-feature
compile matrix, binary-size capture, and install-artifact wording review.

Validation ran from detached worktree:

- Path: `/tmp/kujo-v1-p1-004-clean`
- Commit: `734e406`

## Findings And Fix

The first clean feature-matrix run exposed reduced-build compile failures for
`cargo check --no-default-features` and
`cargo check --no-default-features --features runtime-jit`. Shared interpreter
types referenced optional database, image, and archive crates even when their
features were disabled.

Fix committed in `734e406`:

- gated optional database/image/archive `Value` variants
- gated database pool/connection types and re-exports
- gated formatter, type-name, VM nesting, image method, and cleanup branches
  that mention those optional variants

## Final Clean-Worktree Validation

All final commands passed in the clean worktree:

- `cargo build --release`
- `cargo check --no-default-features`
- `cargo check --no-default-features --features runtime-jit`
- `cargo check --no-default-features --features runtime-db,runtime-image,runtime-archive`
- `bash scripts/measure_binary_size.sh`

Additional focused validation after the feature-gate fix:

- `cargo test --test runtime_path_matrix_contract`

Logs and status manifest:

- `notes/release_evidence/2026-06-19_p1-004/status.tsv`

Binary sizes from `scripts/measure_binary_size.sh`:

- debug: `91,338,800` bytes
- release: `22,596,532` bytes
- release stripped: `22,596,636` bytes

## Install Doc Review

Reviewed:

- `INSTALLATION.md`
- `docs/RELEASE_BINARIES.md`
- `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`
- `docs/RELEASE_ARTIFACT_VALIDATION.md`
- `docs/INSTALL_MATRIX.md`
- `docs/LOCAL_BINARY_INSTALL_GUIDE.md`

Official release docs consistently use tag-based asset names such as
`kujo-<TAG>-linux-x64.tar.gz`, `kujo-<TAG>-macos-x64.tar.gz`,
`kujo-<TAG>-macos-arm64.tar.gz`, and `kujo-<TAG>-windows-x64.zip`.
`docs/LOCAL_BINARY_INSTALL_GUIDE.md` intentionally keeps date+commit artifact
names for unsigned local/test binaries, which is distinct from official release
assets.
