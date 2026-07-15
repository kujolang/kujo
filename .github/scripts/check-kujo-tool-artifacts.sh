#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

artifact_ignore_file="${ARTIFACT_IGNORE_FILE:-config/kujo-tool-artifacts.gitignore}"
repo_gitignore_file="${REPO_GITIGNORE_FILE:-.gitignore}"
base_sha="${1:-${BASE_SHA:-}}"
head_sha="${2:-${HEAD_SHA:-HEAD}}"

if [[ ! -f "$artifact_ignore_file" ]]; then
  echo "[tool-artifacts] ERROR: missing artifact ignore source: $artifact_ignore_file"
  exit 1
fi
if [[ ! -f "$repo_gitignore_file" ]]; then
  echo "[tool-artifacts] ERROR: missing repository gitignore: $repo_gitignore_file"
  exit 1
fi

missing_rules=()
while IFS= read -r rule; do
  if [[ -n "$rule" && "$rule" != \#* ]] && ! grep -Fqx -- "$rule" "$repo_gitignore_file"; then
    missing_rules+=("$rule")
  fi
done < "$artifact_ignore_file"

if (( ${#missing_rules[@]} > 0 )); then
  echo "[tool-artifacts] ERROR: repository gitignore is missing rules from $artifact_ignore_file"
  printf '[tool-artifacts] Missing rule: %s\n' "${missing_rules[@]}"
  exit 1
fi

if [[ -z "$base_sha" || "$base_sha" == "0000000000000000000000000000000000000000" ]]; then
  if git rev-parse --verify HEAD^\{commit\} >/dev/null 2>&1; then
    base_sha="$(git rev-parse HEAD^)"
  else
    echo "[tool-artifacts] OK: ignore rules are present; no commit range to scan"
    exit 0
  fi
fi
if ! git rev-parse --verify "$base_sha^{commit}" >/dev/null 2>&1; then
  echo "[tool-artifacts] ERROR: base SHA is not available locally: $base_sha"
  exit 1
fi
if ! git rev-parse --verify "$head_sha^{commit}" >/dev/null 2>&1; then
  echo "[tool-artifacts] ERROR: head SHA is not available locally: $head_sha"
  exit 1
fi

matcher_dir="$(mktemp -d)"
trap 'rm -rf "$matcher_dir"' EXIT
cp "$artifact_ignore_file" "$matcher_dir/.gitignore"
git -C "$matcher_dir" init -q

violations=()
while IFS= read -r -d '' changed_path; do
  if match="$(git -C "$matcher_dir" check-ignore --no-index -v -- "$changed_path" 2>/dev/null)"; then
    violations+=("$changed_path ($match)")
  fi
done < <(
  git log --format='%H' "$base_sha..$head_sha" |
    while IFS= read -r commit; do
      git diff-tree --root --no-commit-id --name-only -r -m -z "$commit"
    done
)

if (( ${#violations[@]} > 0 )); then
  echo "[tool-artifacts] ERROR: pushed commits contain ignored Kujo tool artifacts"
  printf '[tool-artifacts] Artifact path: %s\n' "${violations[@]}"
  exit 1
fi

echo "[tool-artifacts] OK: all source rules are present and no pushed commit contains a listed artifact"
