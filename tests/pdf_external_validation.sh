#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KUJO="${KUJO:-${ROOT}/target/debug/kujo}"
PDFINFO="${PDFINFO:-$(command -v pdfinfo || true)}"
PDFTOTEXT="${PDFTOTEXT:-$(command -v pdftotext || true)}"
if [[ -z "${PDFINFO}" || -z "${PDFTOTEXT}" ]]; then
    echo "pdf external validation requires Poppler pdfinfo and pdftotext" >&2
    exit 2
fi

TMP_ROOT="$(mktemp -d)"
cleanup() {
    rm -rf "${TMP_ROOT}"
    rm -f "${ROOT}/tests/pdf_external_validation_probe.out"
}
trap cleanup EXIT
export KUJO_PDF_HTML="${ROOT}/tests/fixtures/pdf/hvac-estimate.html"
export KUJO_PDF_OUTPUT="${TMP_ROOT}/hvac-estimate.pdf"

for engine in vm interpreter; do
    rm -f "${KUJO_PDF_OUTPUT}"
    if [[ "${engine}" == "interpreter" ]]; then
        output="$(${KUJO} run "${ROOT}/tests/pdf_external_validation_probe.kujo" --interpreter --allow-fs-read --allow-fs-write --allow-env-read 2>&1)"
    else
        output="$(${KUJO} run "${ROOT}/tests/pdf_external_validation_probe.kujo" --allow-fs-read --allow-fs-write --allow-env-read 2>&1)"
    fi
    "${PDFINFO}" "${KUJO_PDF_OUTPUT}" >"${TMP_ROOT}/${engine}.info"
    "${PDFTOTEXT}" "${KUJO_PDF_OUTPUT}" "${TMP_ROOT}/${engine}.txt"
    python3 - "${TMP_ROOT}/${engine}.info" <<'PY'
import re
import sys

info = open(sys.argv[1], encoding="utf-8").read()
pages = re.search(r"^Pages:\s+(\d+)\s*$", info, re.MULTILINE)
size = re.search(r"^Page size:\s+([\d.]+) x ([\d.]+) pts", info, re.MULTILINE)
assert pages and int(pages[1]) == 1, info
assert size, info
# Poppler versions print different decimal precision. Compare physical A4
# dimensions (210 x 297 mm) to 0.01 pt, rather than requiring rounded '842'.
assert abs(float(size[1]) - 210 * 72 / 25.4) < 0.01, info
assert abs(float(size[2]) - 297 * 72 / 25.4) < 0.01, info
PY
    grep -q 'Northstar Heating' "${TMP_ROOT}/${engine}.txt"
    grep -q '9,850.00' "${TMP_ROOT}/${engine}.txt"
    python3 -c 'import json,sys; value=json.loads(sys.argv[1].splitlines()[-1]); assert value["ok"] and value["pages"] == 1 and value["bytes"] > 1000 and len(value["output_sha256"]) == 64' "${output}"
done

echo "external PDF validation: passed"
