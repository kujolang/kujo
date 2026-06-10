#!/usr/bin/env bash
set -euo pipefail

# Cross-check tracked root files against the canonical policy list in
# docs/REPO_HYGIENE_POLICY.md. This is a fast guard intended to fail before
# broader compile/test work when hygiene drifts.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --root)
      repo_root="${2:?missing value for --root}"
      shift 2
      ;;
    -h|--help)
      cat <<'USAGE'
Usage: bash scripts/repo_hygiene_audit.sh [--root <repo-root>]

Cross-check tracked root files and untracked local clutter against the
canonical policy list in docs/REPO_HYGIENE_POLICY.md.
USAGE
      exit 0
      ;;
    *)
      echo "Repo hygiene audit: unknown option: $1" >&2
      exit 2
      ;;
  esac
done

if [[ "$repo_root" != /* ]]; then
  repo_root="$(cd "$repo_root" && pwd)"
fi

policy_file="${repo_root}/docs/REPO_HYGIENE_POLICY.md"

if [[ ! -f "${policy_file}" ]]; then
  echo "Repo hygiene audit: missing policy file: ${policy_file}" >&2
  exit 1
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

policy_root_files="${tmp_dir}/policy_root_files.txt"
tracked_root_files="${tmp_dir}/tracked_root_files.txt"

awk '
  /Current allowed tracked root files:/ {capture=1; next}
  capture && /^## / {exit}
  capture && /^- `[^`]+`$/ {
    line=$0
    sub(/^- `/, "", line)
    sub(/`$/, "", line)
    print line
  }
' "${policy_file}" | sort > "${policy_root_files}"

if [[ ! -s "${policy_root_files}" ]]; then
  echo "Repo hygiene audit: failed to parse root file allowlist from policy." >&2
  exit 1
fi

git -C "${repo_root}" ls-files | awk -F/ 'NF==1 {print $0}' | sort > "${tracked_root_files}"

if diff -u "${policy_root_files}" "${tracked_root_files}" > "${tmp_dir}/drift.diff"; then
  echo "Repo hygiene audit: tracked root files match docs/REPO_HYGIENE_POLICY.md."
else
  echo "Repo hygiene audit: tracked root surface drifted from docs/REPO_HYGIENE_POLICY.md."
  echo
  cat "${tmp_dir}/drift.diff"
  echo
  echo "Suggested next step: update docs/REPO_HYGIENE_POLICY.md and tests/repo_hygiene_contract.rs together if the drift is intentional."
  exit 1
fi

clutter_candidates="$(
  git -C "${repo_root}" ls-files -o --exclude-standard \
    | awk -F/ 'NF >= 1 { print $1 }' \
    | sort -u
)"

disallowed_clutter=""
while IFS= read -r entry; do
  [[ -z "$entry" ]] && continue
  case "$entry" in
    *.db|*.sqlite|*.sqlite3|*.zip|*.tar|*.tgz|*.tar.gz|*.bak|*.backup|*.orig|*.tmp|*.dmp)
      disallowed_clutter="${disallowed_clutter}${entry}"$'\n'
      ;;
    tmp-*|temp-*|scratch*|backup*|extract*|unzipped*|*_tmp|*_temp|*_backup|*_backup_*|*_extract*)
      disallowed_clutter="${disallowed_clutter}${entry}"$'\n'
      ;;
  esac
done <<< "${clutter_candidates}"

if [[ -n "${disallowed_clutter}" ]]; then
  echo "Repo hygiene audit: disallowed local root clutter detected."
  echo
  printf '%s' "${disallowed_clutter}" | sort -u | sed '/^$/d' | sed 's/^/- /'
  echo
  echo "Allowed local scratch should stay under tmp/ or var/ instead of root-level clutter names."
  exit 1
fi

echo "Repo hygiene audit: tracked root files and untracked root clutter patterns match docs/REPO_HYGIENE_POLICY.md."
