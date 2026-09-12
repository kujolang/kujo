#!/usr/bin/env bash
# Sourced by the two repository fuzz entrypoints after resolving repo_root.
FUZZ_TOOLCHAIN="$(cat "$repo_root/fuzz/RUST_TOOLCHAIN")"
FUZZ_CARGO_VERSION="$(cat "$repo_root/fuzz/CARGO_FUZZ_VERSION")"
if ! [[ "$FUZZ_TOOLCHAIN" =~ ^nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] ||
   ! [[ "$FUZZ_CARGO_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo 'Invalid checked-in fuzz toolchain/version pin' >&2
  exit 1
fi

fuzz_lock_hash() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$repo_root/fuzz/Cargo.lock" | awk '{print $1}'
  else
    shasum -a 256 "$repo_root/fuzz/Cargo.lock" | awk '{print $1}'
  fi
}

fuzz_require_lock() {
  if [[ ! -f "$repo_root/fuzz/Cargo.lock" ]]; then
    echo 'Missing committed fuzz/Cargo.lock; update the fuzz dependency pin explicitly' >&2
    return 1
  fi
  FUZZ_LOCK_SHA="$(fuzz_lock_hash)"
  cargo "+$FUZZ_TOOLCHAIN" metadata --locked --format-version 1 --manifest-path "$repo_root/fuzz/Cargo.toml" >/dev/null
  echo "[evidence] toolchain=$FUZZ_TOOLCHAIN cargo-fuzz=$FUZZ_CARGO_VERSION lock_sha256=$FUZZ_LOCK_SHA"
}

fuzz_check_lock() {
  if [[ "$(fuzz_lock_hash)" != "$FUZZ_LOCK_SHA" ]]; then
    echo 'Fuzz dependency lock changed during execution; campaign is not reproducible' >&2
    return 1
  fi
}
