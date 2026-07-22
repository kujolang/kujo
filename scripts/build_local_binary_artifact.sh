#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

usage() {
	cat <<'EOF'
Usage: bash scripts/build_local_binary_artifact.sh [--install] [--install-dir <dir>] [--version <version>]

Builds a local release binary, packages it as a platform archive, writes a SHA-256
checksum, and optionally installs the binary into a user directory.

Options:
  --install              Copy the built binary into the install directory.
  --install-dir <dir>    Install destination for --install. Default: $HOME/.local/bin
  --version <version>    Override the artifact version prefix. Default: Cargo package version.
  --help                 Show this help text.
EOF
}

install_binary=0
install_dir="${HOME}/.local/bin"
artifact_version=""

while [[ $# -gt 0 ]]; do
	case "$1" in
		--install)
			install_binary=1
			shift
			;;
		--install-dir)
			install_dir="$2"
			shift 2
			;;
		--version)
			artifact_version="$2"
			shift 2
			;;
		--help|-h)
			usage
			exit 0
			;;
		*)
			echo "[local-artifact] ERROR: unknown argument: $1" >&2
			usage >&2
			exit 1
			;;
	esac
done

if command -v sha256sum >/dev/null 2>&1; then
	checksum_cmd=(sha256sum)
elif command -v shasum >/dev/null 2>&1; then
	checksum_cmd=(shasum -a 256)
else
	echo "[local-artifact] ERROR: no SHA-256 tool found (sha256sum/shasum)" >&2
	exit 1
fi

os_name="$(uname -s)"
arch_name="$(uname -m)"

case "$os_name:$arch_name" in
	Darwin:x86_64)
		platform="macos-x64"
		binary_name="kujo"
		archive_ext="tar.gz"
		;;
	Darwin:arm64)
		platform="macos-arm64"
		binary_name="kujo"
		archive_ext="tar.gz"
		;;
	Linux:x86_64)
		platform="linux-x64"
		binary_name="kujo"
		archive_ext="tar.gz"
		;;
	*)
		echo "[local-artifact] ERROR: unsupported local platform: ${os_name} ${arch_name}" >&2
		exit 1
		;;
esac

if [[ -z "$artifact_version" ]]; then
	artifact_version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)"
fi

if [[ -z "$artifact_version" ]]; then
	echo "[local-artifact] ERROR: unable to determine package version from Cargo.toml" >&2
	exit 1
fi

build_date="$(date +%Y%m%d)"
commit_sha="$(git rev-parse --short HEAD)"
artifact_root="target/local-artifacts"
dist_dir="${artifact_root}/dist"
archive_name="kujo-v${artifact_version}-${build_date}-${commit_sha}-${platform}.${archive_ext}"
archive_path="${artifact_root}/${archive_name}"
checksum_path="${archive_path}.sha256"
binary_path="target/release/${binary_name}"

mkdir -p "$artifact_root" "$dist_dir"
rm -f "${dist_dir}/${binary_name}" "$archive_path" "$checksum_path"

echo "[local-artifact] building release binary"
cargo build --release --locked

if [[ ! -x "$binary_path" ]]; then
	echo "[local-artifact] ERROR: expected binary not found at ${binary_path}" >&2
	exit 1
fi

echo "[local-artifact] smoke testing release binary"
"$binary_path" --version
"$binary_path" run examples/hello.kujo

cp "$binary_path" "${dist_dir}/${binary_name}"

echo "[local-artifact] packaging ${archive_name}"
tar -czf "$archive_path" -C "$dist_dir" "$binary_name"

echo "[local-artifact] writing checksum"
(
	cd "$artifact_root"
	"${checksum_cmd[@]}" "$archive_name" > "${archive_name}.sha256"
)

if [[ "$install_binary" -eq 1 ]]; then
	mkdir -p "$install_dir"
	cp "$binary_path" "${install_dir}/kujo"
	chmod +x "${install_dir}/kujo"
	echo "[local-artifact] installed ${install_dir}/kujo"
	"${install_dir}/kujo" --version
fi

echo "[local-artifact] artifact: ${archive_path}"
echo "[local-artifact] checksum: ${checksum_path}"
