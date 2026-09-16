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
trap 'rm -rf "${TMP_ROOT}"' EXIT
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
    grep -q 'Pages:.*1' "${TMP_ROOT}/${engine}.info"
    grep -q 'Page size:.*595.*842.*A4' "${TMP_ROOT}/${engine}.info"
    grep -q 'Northstar Heating' "${TMP_ROOT}/${engine}.txt"
    grep -q '9,850.00' "${TMP_ROOT}/${engine}.txt"
    python3 -c 'import json,sys; value=json.loads(sys.argv[1].splitlines()[-1]); assert value["ok"] and value["pages"] == 1 and value["bytes"] > 1000 and len(value["output_sha256"]) == 64' "${output}"
done

echo "external PDF validation: passed"
