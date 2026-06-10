#!/usr/bin/env bash
set -euo pipefail

mode="full"

usage() {
  cat <<'USAGE'
Usage: bash scripts/release_candidate_gate.sh [--full|--roadmap-only|--help]

Modes:
  --full          Run roadmap P0/P1 checks, then execute full release gate commands.
  --roadmap-only  Only validate roadmap readiness preconditions.

Notes:
  - This script is for v1.0 release-candidate readiness checks.
  - `--full` delegates core verification to scripts/release_gate.sh --full.
USAGE
}

if [[ "$#" -gt 1 ]]; then
  usage
  exit 2
fi

if [[ "$#" -eq 1 ]]; then
  case "$1" in
    --full)
      mode="full"
      ;;
    --roadmap-only)
      mode="roadmap-only"
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
fi

run_cmd() {
  echo ""
  echo "+ $*"
  "$@"
}

check_p0_p1_roadmap_readiness() {
  local roadmap="ROADMAP.md"
  if [[ ! -f "$roadmap" ]]; then
    echo "Missing $roadmap" >&2
    return 1
  fi

  local unresolved
  unresolved="$({
    awk '
      BEGIN { pending = 0; id = ""; desc = "" }
      /^\[ \] V1-/ {
        pending = 1
        id = $2
        gsub(":", "", id)
        desc = $0
        next
      }
      pending == 1 && /Priority: P0/ {
        print id "|P0|" desc
        pending = 0
        next
      }
      pending == 1 && /Priority: P1/ {
        if (id != "V1-REL-001") {
          print id "|P1|" desc
        }
        pending = 0
        next
      }
      /^\[x\] V1-/ {
        pending = 0
      }
    ' "$roadmap"
  } || true)"

  if [[ -n "$unresolved" ]]; then
    echo "Unresolved P0/P1 roadmap items detected:" >&2
    while IFS='|' read -r item priority line; do
      [[ -z "$item" ]] && continue
      echo "  - ${item} (${priority}): ${line}" >&2
    done <<< "$unresolved"
    return 1
  fi

  echo "Roadmap check: no unresolved P0/P1 implementation items (excluding active V1-REL-001)."
}

check_release_checklist_section_exists() {
  local pattern="Final v1.0 Release Checklist"
  if command -v rg >/dev/null 2>&1; then
    if rg -q "$pattern" ROADMAP.md; then
      return 0
    fi
  elif grep -q "$pattern" ROADMAP.md; then
    return 0
  fi

  echo "ROADMAP.md is missing final release checklist section" >&2
  return 1
}

run_roadmap_checks() {
  run_cmd check_p0_p1_roadmap_readiness
  run_cmd check_release_checklist_section_exists
}

run_roadmap_checks

if [[ "$mode" == "full" ]]; then
  run_cmd bash scripts/release_gate.sh --full
  run_cmd cargo test --test serve_command_integration -- --test-threads=1
fi

echo ""
echo "release-candidate gate checks (${mode}) completed"
