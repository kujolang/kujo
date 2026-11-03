# Base64 port state audit — partial

Follow-up: the encoding preview rule is now corrected in `src/10_basic.kujo`.
The reusable collector fails on the original port with 15 checksum mismatches
and passes on the corrected port with none, across all three runtimes. The
original audit archive below remains historical evidence; the separate
`base64-preview-fix-2026-11-03.json.gz` retains the before/after regression data.
No encoding/decoding work, registered input or iteration count changed.
The corrected generated port also passes all 150 registered test-size checksums
on the locally verified loop-fix runtime (50 each in VM, interpreter and JIT).
Its complete generated text matches the ordered source fragments, and the
archive retains exact configs, outputs, binary/source hashes and verification.
This does not extend production or comparator-execution coverage.

The unchanged port's encode/decode functions pass full-data checks at 12 input
sizes in VM, interpreter and JIT: 0, 1, 2, 3, 4, 23, 30, 255, 256, 257, 1,024
and 500,000 bytes. Each input consists of ASCII `a`, matching the registered
workloads. Each direction runs twice, checking the accumulated length result.
This is 36 input/runtime pairs for each direction, not an official production
run: production uses 5,000 encoding or 4,000 decoding iterations.

The collector calls the existing `base64_prepare`, `base64_run` and
`base64_checksum` functions, extracted with the unchanged port prelude. Complete
encoded output is compared with Ruby's standard Base64 implementation; decoded
values are compared byte-for-byte with the original input in Kujo. Independent
verification decodes every captured encoded buffer back to its exact input.

## Confirmed checksum preview difference

For input sizes 0–4, Kujo's encoding checksum differs from the formula in the
pinned Rust and Goose sources in all three Kujo runtimes. Kujo unconditionally
appends `...` to both previews. Upstream appends it only when that particular
buffer exceeds four bytes. The encoded data and accumulated length still match.

This is a port-formatting gap, not an encoder/runtime failure. Registered
test-size inputs are 30 bytes for encoding and 23 for decoding; both production
inputs are 500,000 bytes. Their encoding checksum previews match. No registered
configuration or port code was changed by the original audit. The subsequent
preview-only fix is separate from runtime work and uses this short-input
regression coverage. The decoder's short-input checksum boundary remains open.

The upstream Rust decoder checksum slices its decoded buffer at four bytes,
so this audit does not claim a shared decoder-checksum contract below that size.
Decoded contents and accumulated lengths are checked for all 12 sizes.

## Library and evidence boundaries

Rust calls `base64_simd::STANDARD`; Kujo calls
`base64::engine::general_purpose::STANDARD` through its native builtins. Goose
calls its Base64 library. This source distinction does not quantify the speed
gap or establish that changing libraries would eliminate it. The registered
small inputs repeat 8,192 times, so call/state overhead also needs attribution.

Pinned upstream: `3c9f84ddaeec834c0631747cb789e8cfd602f993`; all four comparator
source files were checked against that Git revision. Kujo uses the frozen
accepted CLI with SHA-256
`bf5bd666143cabb2339a72a5635702a6c735639e25d12663396aea7a87abaed4`.

`probes/audit-base64-state.rb UPSTREAM FROZEN_BINARY NEW_OUTPUT` reproduces the
checks. `base64-state-audit-2026-11-03.json.gz` retains the collector, source
identities, generated fixture, complete raw outputs and report. Initial local
exploration is separate from the final reusable-collector evidence in
`.audit-evidence/native-performance-campaign-2026-11-03/base64-state-verified/`.

Both ledger rows remain open: execute the Rust/Goose full-state comparators,
audit non-ASCII byte inputs and malformed-input contracts, count work/copies and
complete unchanged production iteration counts. This is neither native-coverage
proof nor a timing result. No Kostya contact or benchmark submission occurred.
