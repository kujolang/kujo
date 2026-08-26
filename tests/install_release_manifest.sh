#!/usr/bin/env bash
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
temp_root="$(mktemp -d "${TMPDIR:-/tmp}/kujo-installer-test.XXXXXX")"
trap 'rm -rf "$temp_root"' EXIT

manifest="$temp_root/dispatch.refs"
cat > "$manifest" <<'EOF'
# Exact dependency closure for Dispatch.
kujo=v1.0.2
ai-sdk=849dbbbba7a734938320dd9569d1ed7aa6240298
agents-sdk=d3904d348754b492bda298b6c30f49c1eb24b7ea
dispatch=v1.1.0
EOF

output="$(bash "$repo_root/install.sh" --dry-run --source --package dispatch --release-manifest "$manifest" --prefix "$temp_root/root" --bin-dir "$temp_root/bin")"
for expected in \
	"would build Kujo from source at ref v1.0.2" \
	"would download kujolang/ai-sdk at 849dbbbba7a734938320dd9569d1ed7aa6240298" \
	"would download kujolang/agents-sdk at d3904d348754b492bda298b6c30f49c1eb24b7ea" \
	"would download kujolang/dispatch at v1.1.0"; do
	grep -F "$expected" <<<"$output" >/dev/null || {
		echo "missing expected installer output: $expected" >&2
		exit 1
	}
done
if grep -F "kujolang/watchdog" <<<"$output" >/dev/null; then
	echo "single-package install unexpectedly selected unrelated AI profile repositories" >&2
	exit 1
fi

missing_manifest="$temp_root/missing.refs"
printf '%s\n' 'kujo=v1.0.2' 'ai-sdk=abc12345' 'agents-sdk=def67890' > "$missing_manifest"
if bash "$repo_root/install.sh" --dry-run --source --package dispatch --release-manifest "$missing_manifest" --prefix "$temp_root/missing-root" --bin-dir "$temp_root/missing-bin" >"$temp_root/missing.out" 2>&1; then
	echo "installer accepted a release manifest without the Dispatch pin" >&2
	exit 1
fi
grep -F "missing required repository pin: dispatch" "$temp_root/missing.out" >/dev/null

duplicate_manifest="$temp_root/duplicate.refs"
printf '%s\n' 'kujo=v1.0.2' 'kujo=v1.0.1' > "$duplicate_manifest"
if bash "$repo_root/install.sh" --dry-run --source --package dispatch --release-manifest "$duplicate_manifest" --prefix "$temp_root/duplicate-root" --bin-dir "$temp_root/duplicate-bin" >"$temp_root/duplicate.out" 2>&1; then
	echo "installer accepted duplicate repository pins" >&2
	exit 1
fi
grep -F "duplicate repository ref override: kujo" "$temp_root/duplicate.out" >/dev/null

if bash "$repo_root/install.sh" --dry-run --source --package dispatch --release-manifest http://example.invalid/refs >"$temp_root/http.out" 2>&1; then
	echo "installer accepted an insecure remote manifest URL" >&2
	exit 1
fi
grep -F "release manifests must use HTTPS" "$temp_root/http.out" >/dev/null

echo "installer release-manifest contract passed"
