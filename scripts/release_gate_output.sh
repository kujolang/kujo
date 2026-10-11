#!/usr/bin/env bash
# Shared by the release gate and its output-contract tests.
# Default output is unchanged; opt-in logs retain complete stdout and stderr.
gate_log_directory=""
gate_command_index=0
if [[ -n "${KUJO_RELEASE_GATE_LOG_DIR:-}" ]]; then
  mkdir -p -- "${KUJO_RELEASE_GATE_LOG_DIR}"
  gate_log_directory="$(mktemp -d "${KUJO_RELEASE_GATE_LOG_DIR%/}/kujo-release.XXXXXXXX")"
fi

run_cmd() {
  echo ""
  echo "+ $*"
  if [[ -z "${gate_log_directory}" ]]; then
    "$@"
    return
  fi

  gate_command_index=$((gate_command_index + 1))
  local log_file="${gate_log_directory}/${gate_command_index}.log"
  local status=0
  # The private directory prevents other users reading potentially sensitive
  # diagnostics. A separate file per command preserves evidence on failure.
  if "$@" >"${log_file}" 2>&1; then
    status=0
  else
    status=$?
  fi
  printf 'exit=%s log=%s\n' "${status}" "${log_file}"
  return "${status}"
}
