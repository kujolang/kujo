# Production Base64 verification across runtimes

Base64 encoding and decoding pass unchanged production checksums in VM,
interpreter and JIT on accepted runtime `fd7b764`. Each uses size 500,000.
Encoding uses 1,000 warmups and 5,000 measured iterations; decoding uses
800 warmups and 4,000 measured iterations. No runtime code or workload changed.

| Case | Runtime | Verified checksum | Total elapsed | Peak RSS bytes |
| --- | --- | ---: | ---: | ---: |
| Base64::Encode | jit | 3516658482 | 2.113 s | 31547392 |
| Base64::Decode | jit | 1329134018 | 2.513 s | 31059968 |
| Base64::Encode | vm | 3516658482 | 2.357 s | 17838080 |
| Base64::Decode | vm | 1329134018 | 2.238 s | 17801216 |
| Base64::Encode | interpreter | 3516658482 | 2.016 s | 16494592 |
| Base64::Decode | interpreter | 1329134018 | 2.295 s | 16232448 |

Single observations on the loaded local machine establish completion and observed
resources, not controlled runtime comparisons or speedups. Each child ran alone
within this verification sequence. Total elapsed includes setup and warmup.

Independent verification checks exact case/configuration inventories, source and
accepted executable hashes, exits, untruncated output, checksum and summary lines,
complete monotonic phase checkpoints and child peak RSS. JIT measurements show
native entries, without implying native CPU coverage. VM measurements also
pass schema/hash checks and report zero native JIT entries. Interpreter execution
uses `--interpreter`; VM measurement instrumentation is unsupported there and was
not requested. Runner commands and reports preserve that distinction.

The bundle `accepted-production-base64-parity-2026-10-31.json.gz` contains
55 digest-verified artifacts, including raw results, runners, verifiers,
measurements where supported and host metadata. Accepted executable SHA-256:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.

With the earlier sorting, JSON and CSV passes, current production coverage is 20/150 runtime/case
combinations. The remaining 130, platform verification and clean-host comparison
remain open. No Kostya contact occurred.
