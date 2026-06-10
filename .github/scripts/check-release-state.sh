#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

cargo_version="$(grep -E '^version[[:space:]]*=[[:space:]]*"' Cargo.toml | head -n 1 | sed -E 's/^version[[:space:]]*=[[:space:]]*"([0-9]+\.[0-9]+\.[0-9]+)".*/\1/')"
if [[ -z "$cargo_version" ]]; then
	echo "[release-state] ERROR: Could not parse version from Cargo.toml"
	exit 1
fi

expected_readme="$(printf 'The project is currently at `%s` in `Cargo.toml`' "$cargo_version")"
if ! grep -Fq "$expected_readme" README.md; then
	echo "[release-state] ERROR: README.md does not reflect Cargo.toml version $cargo_version"
	echo "[release-state] Expected to find: $expected_readme"
	exit 1
fi

echo "[release-state] README.md matches Cargo.toml version: $cargo_version"

expected_roadmap_line="$(printf '> Current crate version: `%s` in [Cargo.toml](Cargo.toml)' "$cargo_version")"
if ! grep -Fq "$expected_roadmap_line" ROADMAP.md; then
	echo "[release-state] ERROR: ROADMAP.md current crate version line does not match Cargo.toml version $cargo_version"
	echo "[release-state] Expected to find: $expected_roadmap_line"
	exit 1
fi

echo "[release-state] ROADMAP.md current crate version matches Cargo.toml version: $cargo_version"

echo "[release-state] OK: release status files are consistent with Cargo.toml"
