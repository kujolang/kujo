#!/usr/bin/env bash
set -euo pipefail

version="${1:?Kujo release version is required}"
install_dir="${2:?Install directory is required}"

[[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
  echo "setup-kujo: version must be an exact vMAJOR.MINOR.PATCH release tag" >&2
  exit 2
}

case "$(uname -s):$(uname -m)" in
  Linux:x86_64) platform="linux-x64" ;;
  Darwin:x86_64) platform="macos-x64" ;;
  Darwin:arm64) platform="macos-arm64" ;;
  *) echo "setup-kujo: unsupported POSIX runner $(uname -s) $(uname -m)" >&2; exit 2 ;;
esac

asset="kujo-${version}-${platform}.tar.gz"
release_url="https://github.com/kujolang/kujo/releases/tag/${version}"
base_url="https://github.com/kujolang/kujo/releases/download/${version}"
binary_path="${install_dir}/kujo"
metadata_path="${install_dir}/setup-kujo.env"

mkdir -p "$install_dir"
if [[ ! -x "$binary_path" || ! -f "$metadata_path" ]]; then
  work_dir="$(mktemp -d)"
  trap 'rm -rf "$work_dir"' EXIT
  curl --proto '=https' --tlsv1.2 --fail --location --retry 3 --output "${work_dir}/${asset}" "${base_url}/${asset}"
  curl --proto '=https' --tlsv1.2 --fail --location --retry 3 --output "${work_dir}/${asset}.sha256" "${base_url}/${asset}.sha256"
  expected="$(awk 'NR == 1 { print tolower($1) }' "${work_dir}/${asset}.sha256")"
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "${work_dir}/${asset}" | awk '{ print tolower($1) }')"
  elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "${work_dir}/${asset}" | awk '{ print tolower($1) }')"
  else
    echo "setup-kujo: sha256sum or shasum is required" >&2
    exit 2
  fi
  [[ "$expected" =~ ^[0-9a-f]{64}$ ]] || { echo "setup-kujo: release checksum is malformed" >&2; exit 1; }
  [[ "$actual" == "$expected" ]] || { echo "setup-kujo: checksum mismatch for ${asset}" >&2; exit 1; }
  tar -xzf "${work_dir}/${asset}" -C "$install_dir"
  chmod 0755 "$binary_path"
  printf 'version=%s\nchecksum=%s\nasset=%s\nprovenance_url=%s\n' "$version" "$expected" "$asset" "$release_url" > "$metadata_path"
fi

source "$metadata_path"
[[ "${version:-}" == "$1" ]] || { echo "setup-kujo: cached runtime version mismatch" >&2; exit 1; }
"$binary_path" --version | grep -F "${version#v}" >/dev/null || { echo "setup-kujo: installed binary version mismatch" >&2; exit 1; }
printf '%s\n' "$install_dir" >> "$GITHUB_PATH"
printf 'kujo-path=%s\nchecksum=%s\nasset=%s\nprovenance-url=%s\n' "$binary_path" "$checksum" "$asset" "$provenance_url" >> "$GITHUB_OUTPUT"
