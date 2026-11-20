#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

if command -v sha256sum >/dev/null 2>&1; then
	checksum_cmd="sha256sum"
	verify_cmd="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
	checksum_cmd="shasum -a 256"
	verify_cmd="shasum -a 256"
else
	echo "[artifact-validate] ERROR: no SHA-256 tool found (sha256sum/shasum)"
	exit 1
fi

echo "[artifact-validate] building release binary"
cargo build --release

install_root="$(mktemp -d)/kujo-install-root"

echo "[artifact-validate] installing from clean root: $install_root"
cargo install --path . --root "$install_root" --force --locked --offline

kujo_bin="$install_root/bin/kujo"
if [[ ! -x "$kujo_bin" ]]; then
	echo "[artifact-validate] ERROR: expected binary not found at $kujo_bin"
	exit 1
fi

echo "[artifact-validate] running installed binary checks"
"$kujo_bin" --version
"$kujo_bin" run examples/hello.kujo

checksum_file="target/release/kujo.sha256"

echo "[artifact-validate] generating checksum: $checksum_file"
(
	cd target/release
	$checksum_cmd kujo > kujo.sha256
)

echo "[artifact-validate] verifying checksum"
(
	cd target/release
	$verify_cmd -c kujo.sha256
)

artifact_root="target/release-artifacts"
mkdir -p "$artifact_root"
cp target/release/kujo "$artifact_root/kujo"
tarball="$artifact_root/kujo-local.tar.gz"

echo "[artifact-validate] packaging local release tarball: $tarball"
tar -czf "$tarball" -C "$artifact_root" kujo

echo "[artifact-validate] generating tarball checksum"
(
	cd "$artifact_root"
	$checksum_cmd kujo-local.tar.gz > kujo-local.tar.gz.sha256
	$verify_cmd -c kujo-local.tar.gz.sha256
)

extract_root="$(mktemp -d)/kujo-artifact-extract"
mkdir -p "$extract_root"
tar -xzf "$tarball" -C "$extract_root"

echo "[artifact-validate] validating repository-independent artifact run"
"$extract_root/kujo" --version

artifact_script="$extract_root/hello.kujo"
cat > "$artifact_script" <<'KUJO_SCRIPT'
print("artifact-ok")
KUJO_SCRIPT

"$extract_root/kujo" run "$artifact_script"

echo "[artifact-validate] OK: install flow and checksum verification passed"
