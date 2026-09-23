#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

cargo_version="$(grep -E '^version[[:space:]]*=[[:space:]]*"' Cargo.toml | head -n 1 | sed -E 's/^version[[:space:]]*=[[:space:]]*"([0-9]+\.[0-9]+\.[0-9]+)".*/\1/')"
if [[ -z "$cargo_version" ]]; then
	echo "[release-state] ERROR: Could not parse version from Cargo.toml"
	exit 1
fi

rust_version="$(grep -E '^rust-version[[:space:]]*=[[:space:]]*"' Cargo.toml | head -n 1 | sed -E 's/^rust-version[[:space:]]*=[[:space:]]*"([0-9]+\.[0-9]+)".*/\1/')"
if [[ -z "$rust_version" ]]; then
	echo "[release-state] ERROR: Cargo.toml must declare rust-version" >&2
	exit 1
fi

expected_readme="$(printf 'The source tree is currently at `%s` in `Cargo.toml`' "$cargo_version")"
if ! grep -Fq "$expected_readme" README.md; then
	echo "[release-state] ERROR: README.md does not reflect Cargo.toml version $cargo_version"
	echo "[release-state] Expected to find: $expected_readme"
	exit 1
fi

echo "[release-state] README.md matches Cargo.toml version: $cargo_version"

latest_stable_tag="$(git tag --list 'v*' | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -n 1)"
if [[ -z "$latest_stable_tag" ]]; then
	echo "[release-state] ERROR: Could not resolve the latest stable version tag"
	exit 1
fi

expected_stable_release="$(printf 'latest published stable release tag is `%s`' "$latest_stable_tag")"
if ! grep -Fq "$expected_stable_release" README.md; then
	echo "[release-state] ERROR: README.md does not identify latest stable tag $latest_stable_tag"
	echo "[release-state] Expected to find: $expected_stable_release"
	exit 1
fi

echo "[release-state] README.md matches latest stable tag: $latest_stable_tag"

expected_roadmap_line="$(printf '> Current crate version: `%s` in [Cargo.toml](Cargo.toml)' "$cargo_version")"
if ! grep -Fq "$expected_roadmap_line" ROADMAP.md; then
	echo "[release-state] ERROR: ROADMAP.md current crate version line does not match Cargo.toml version $cargo_version"
	echo "[release-state] Expected to find: $expected_roadmap_line"
	exit 1
fi

echo "[release-state] ROADMAP.md current crate version matches Cargo.toml version: $cargo_version"

expected_roadmap_release="$(printf 'Stable release: [v%s](https://github.com/kujolang/kujo/releases/tag/v%s)' "$cargo_version" "$cargo_version")"
expected_install_release="$(printf 'Kujo %s is the current stable native release.' "$cargo_version")"
expected_installer_default="$(printf 'DEFAULT_RELEASE_VERSION="${KUJO_RELEASE_VERSION:-v%s}"' "$cargo_version")"
expected_binary_example="$(printf 'KUJO_VERSION="v%s"' "$cargo_version")"

for check in \
	"ROADMAP.md|$expected_roadmap_release|stable release" \
	"INSTALLATION.md|$expected_install_release|installation release" \
	"install.sh|$expected_installer_default|installer default" \
	".github/actions/setup-kujo/action.yml|    default: v$cargo_version|setup action default" \
	"docs/SETUP_KUJO_ACTION.md|    version: v$cargo_version|setup action example" \
	"docs/BUILD_AN_AGENT.md|Install the stable Kujo v$cargo_version runtime|agent guide runtime" \
	"docs/RELEASE_BINARIES.md|$expected_binary_example|release binary example"; do
	IFS='|' read -r path needle label <<<"$check"
	if ! grep -Fq "$needle" "$path"; then
		echo "[release-state] ERROR: $label mismatch in $path" >&2
		echo "[release-state] Expected to find: $needle" >&2
		exit 1
	fi
done

for manifest in npm/package.json npm/runtime/package.json npm/platforms/*/package.json; do
	manifest_version="$(node -p "require('./${manifest}').version")"
	if [[ "$manifest_version" != "$cargo_version" ]]; then
		echo "[release-state] ERROR: $manifest version $manifest_version does not match Cargo $cargo_version" >&2
		exit 1
	fi
done

for check in \
	"README.md|Kujo requires Rust ${rust_version} or newer|README MSRV" \
	"INSTALLATION.md|Rust ${rust_version} or newer|installation MSRV" \
	"docs/RELEASE_ARTIFACT_VALIDATION.md|Rust \`${rust_version}+\`|release validation MSRV"; do
	IFS='|' read -r path needle label <<<"$check"
	if ! grep -Fq "$needle" "$path"; then
		echo "[release-state] ERROR: $label mismatch in $path" >&2
		echo "[release-state] Expected to find: $needle" >&2
		exit 1
	fi
done

echo "[release-state] OK: development and stable release states are consistent"
