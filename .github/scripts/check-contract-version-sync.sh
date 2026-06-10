#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

expected_status="Status: v1.0.0 baseline draft (active)"
expected_contract_version="1.0.0-draft"

require_line() {
	local file_path="$1"
	local needle="$2"
	local label="$3"

	if ! grep -Fq "$needle" "$file_path"; then
		echo "[contract-sync] ERROR: ${label} mismatch in ${file_path}"
		echo "[contract-sync] Expected line: ${needle}"
		exit 1
	fi

	echo "[contract-sync] OK: ${label} in ${file_path}"
}

require_line "docs/CLI_MACHINE_READABLE_CONTRACTS.md" "$expected_status" "status"
require_line "docs/CLI_MACHINE_READABLE_CONTRACTS.md" "Contract version: \`${expected_contract_version}\`" "contract version"

require_line "docs/PROTOCOL_CONTRACTS.md" "$expected_status" "status"
require_line "docs/PROTOCOL_CONTRACTS.md" "Contract version: \`${expected_contract_version}\`" "contract version"

require_line "docs/LANGUAGE_SPEC.md" "$expected_status" "status"
require_line "docs/LANGUAGE_SPEC.md" "Spec version: ${expected_contract_version}" "spec version"

echo "[contract-sync] OK: CLI/LSP/language contract metadata is aligned"
