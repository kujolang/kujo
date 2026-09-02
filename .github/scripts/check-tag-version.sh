#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

cargo_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
if [[ -z "$cargo_version" ]]; then
	echo "[tag-version] ERROR: Could not parse Cargo.toml version" >&2
	exit 1
fi

expected_tag="v${cargo_version}"
release_tag="${KUJO_RELEASE_TAG:-${GITHUB_REF_NAME:-}}"
ref_type="${GITHUB_REF_TYPE:-}"

if [[ "$ref_type" == "tag" || -n "${KUJO_RELEASE_TAG:-}" ]]; then
	if [[ "$release_tag" != "$expected_tag" ]]; then
		echo "[tag-version] ERROR: release tag ${release_tag:-<empty>} does not match Cargo version ${expected_tag}" >&2
		exit 1
	fi
fi

manifest_paths=(npm/package.json npm/runtime/package.json npm/platforms/*/package.json)
for manifest_path in "${manifest_paths[@]}"; do
	manifest_version="$(node -e 'const fs=require("fs"); process.stdout.write(JSON.parse(fs.readFileSync(process.argv[1], "utf8")).version)' "$manifest_path")"
	if [[ "$manifest_version" != "$cargo_version" ]]; then
		echo "[tag-version] ERROR: $manifest_path version $manifest_version does not match Cargo version $cargo_version" >&2
		exit 1
	fi
done

echo "[tag-version] OK: Cargo, npm manifests, and release tag agree on $cargo_version"
