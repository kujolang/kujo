#!/usr/bin/env bash
set -euo pipefail

TARGET_FILE="src/jit.rs"
STRICT=1

usage() {
  cat <<'USAGE'
Usage: scripts/check_jit_safety_contracts.sh [options]

Validates that executable unsafe boundaries in a Rust source file have an attached
SAFETY contract block using the canonical schema.

Canonical schema:
  // SAFETY:
  // - Preconditions: <ownership/lifetime/ABI assumptions>
  // - Postconditions: <state/result guarantees>

Executable unsafe boundaries matched:
  - unsafe extern "C" fn ...
  - unsafe fn ...
  - unsafe { ... }

Options:
  --file <path>      Rust file to validate (default: src/jit.rs)
  --allow-missing    Do not fail on violations; print summary and exit 0.
  --help             Show this help message.

Exit codes:
  0 success
  1 usage error
  2 violations found (strict mode)
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --file)
      if [[ $# -lt 2 ]]; then
        echo "--file requires a path argument" >&2
        exit 1
      fi
      TARGET_FILE="$2"
      shift 2
      ;;
    --allow-missing)
      STRICT=0
      shift
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "unsupported argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ ! -f "$TARGET_FILE" ]]; then
  echo "target file not found: $TARGET_FILE" >&2
  exit 1
fi

awk -v strict="$STRICT" -v target_file="$TARGET_FILE" '
  {
    lines[NR] = $0
  }

  END {
    boundaries = 0
    violations = 0

    for (i = 1; i <= NR; i++) {
      line = lines[i]
      is_boundary = (line ~ /unsafe[[:space:]]+extern[[:space:]]+"C"[[:space:]]+fn/)
      if (line ~ /^[[:space:]]*pub[[:space:]]+type[[:space:]]/ || line ~ /=/) {
        is_boundary = 0
      }
      is_boundary = is_boundary || (line ~ /(^|[^[:alnum:]_])unsafe[[:space:]]+fn[[:space:]]/)
      is_boundary = is_boundary || (line ~ /unsafe[[:space:]]*\{/)
      if (is_boundary) {
        boundaries++
        found_safety = 0
        found_pre = 0
        found_post = 0

        start = i - 8
        if (start < 1) {
          start = 1
        }

        for (j = i - 1; j >= start; j--) {
          prev = lines[j]

          if (prev ~ /^[[:space:]]*$/) {
            continue
          }

          if (prev !~ /^[[:space:]]*\/\//) {
            break
          }

          if (prev ~ /SAFETY:/) {
            found_safety = 1
          }
          if (prev ~ /Preconditions:/) {
            found_pre = 1
          }
          if (prev ~ /Postconditions:/) {
            found_post = 1
          }
        }

        if (!(found_safety && found_pre && found_post)) {
          violations++
          printf("missing SAFETY contract: %s:%d: %s\n", target_file, i, line) > "/dev/stderr"
        }
      }
    }

    printf("Checked %d executable unsafe boundaries in %s; missing contracts: %d\n", boundaries, target_file, violations)

    if (strict == 1 && violations > 0) {
      exit 2
    }
  }
' "$TARGET_FILE"
