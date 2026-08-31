#!/usr/bin/env bash
set -euo pipefail

action_dir=".github/actions/setup-kujo"
test -f "${action_dir}/action.yml"
test -f "${action_dir}/install.sh"
test -f "${action_dir}/install.ps1"
bash -n "${action_dir}/install.sh"

grep -F "actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830" "${action_dir}/action.yml" >/dev/null
grep -F 'curl --proto' "${action_dir}/install.sh" >/dev/null
grep -F 'sha256sum' "${action_dir}/install.sh" >/dev/null
grep -F 'Get-FileHash -Algorithm SHA256' "${action_dir}/install.ps1" >/dev/null
grep -F 'windows-x64' "${action_dir}/install.ps1" >/dev/null
grep -F 'provenance-url' "${action_dir}/action.yml" >/dev/null
grep -F 'x64-windows-static-md' docs/SETUP_KUJO_ACTION.md >/dev/null

if command -v ruby >/dev/null 2>&1; then
  ruby -e 'require "yaml"; action = YAML.load_file(ARGV[0]); abort "missing composite runs" unless action.dig("runs", "using") == "composite"' "${action_dir}/action.yml"
fi

echo "setup-kujo action contract: ok"
